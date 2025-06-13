#!/usr/bin/env python3
"""
后端健康检查工具
使用backend参数直接测试各个后端的可用性
"""

import requests
import json
import time
import argparse
from datetime import datetime
from typing import Dict, List, Optional, Tuple
from concurrent.futures import ThreadPoolExecutor, as_completed

class BackendHealthChecker:
    def __init__(self, base_url: str = "http://localhost:3000", auth_token: str = "test-token"):
        self.base_url = base_url
        self.auth_token = auth_token
        self.session = requests.Session()
        self.session.headers.update({
            "Content-Type": "application/json",
            "Authorization": f"Bearer {auth_token}"
        })
    
    def get_available_backends(self) -> List[str]:
        """获取可用的后端列表"""
        try:
            response = self.session.get(f"{self.base_url}/smart-ai/weights")
            if response.status_code == 200:
                data = response.json()
                backends = set()
                for model in data.get("models", []):
                    for backend in model.get("backends", []):
                        if backend.get("enabled", False):
                            backends.add(backend["provider"])
                return sorted(list(backends))
        except Exception as e:
            print(f"⚠️  无法获取后端列表: {e}")
        
        # 返回默认后端列表
        return ["openai_official", "anthropic_claude", "google_gemini"]
    
    def get_available_models(self) -> List[str]:
        """获取可用的模型列表"""
        try:
            response = self.session.get(f"{self.base_url}/models")
            if response.status_code == 200:
                data = response.json()
                return [model["id"] for model in data.get("data", [])]
        except Exception as e:
            print(f"⚠️  无法获取模型列表: {e}")
        
        return ["gpt-4o", "claude-sonnet-4"]
    
    def check_backend_health(
        self, 
        backend: str, 
        model: str = "gpt-4o", 
        timeout: int = 30,
        test_streaming: bool = False
    ) -> Dict:
        """检查单个后端的健康状态"""
        start_time = time.time()
        
        payload = {
            "model": model,
            "messages": [{"role": "user", "content": "ping"}],
            "backend": backend,
            "max_tokens": 5,
            "stream": test_streaming
        }
        
        try:
            response = self.session.post(
                f"{self.base_url}/v1/chat/completions",
                json=payload,
                timeout=timeout,
                stream=test_streaming
            )
            
            response_time = time.time() - start_time
            
            if response.status_code == 200:
                if test_streaming:
                    # 对于流式请求，读取第一个数据块
                    try:
                        first_chunk = next(response.iter_lines(decode_unicode=True))
                        return {
                            "status": "healthy",
                            "backend": backend,
                            "model": model,
                            "response_time": response_time,
                            "http_status": response.status_code,
                            "streaming": True,
                            "first_chunk": first_chunk[:100] if first_chunk else None
                        }
                    except Exception as e:
                        return {
                            "status": "unhealthy",
                            "backend": backend,
                            "model": model,
                            "response_time": response_time,
                            "http_status": response.status_code,
                            "error": f"Streaming error: {e}",
                            "streaming": True
                        }
                else:
                    # 非流式请求
                    data = response.json()
                    content = data.get("choices", [{}])[0].get("message", {}).get("content", "")
                    return {
                        "status": "healthy",
                        "backend": backend,
                        "model": model,
                        "response_time": response_time,
                        "http_status": response.status_code,
                        "content": content[:50] if content else None,
                        "streaming": False
                    }
            else:
                return {
                    "status": "unhealthy",
                    "backend": backend,
                    "model": model,
                    "response_time": response_time,
                    "http_status": response.status_code,
                    "error": response.text[:200],
                    "streaming": test_streaming
                }
                
        except requests.exceptions.Timeout:
            return {
                "status": "timeout",
                "backend": backend,
                "model": model,
                "response_time": timeout,
                "error": "Request timeout",
                "streaming": test_streaming
            }
        except Exception as e:
            return {
                "status": "error",
                "backend": backend,
                "model": model,
                "response_time": time.time() - start_time,
                "error": str(e),
                "streaming": test_streaming
            }
    
    def check_all_backends(
        self, 
        backends: Optional[List[str]] = None,
        model: str = "gpt-4o",
        timeout: int = 30,
        test_streaming: bool = False,
        parallel: bool = True
    ) -> List[Dict]:
        """检查所有后端的健康状态"""
        if backends is None:
            backends = self.get_available_backends()
        
        print(f"🏥 检查 {len(backends)} 个后端的健康状态...")
        print(f"模型: {model}")
        print(f"超时: {timeout}s")
        print(f"流式测试: {'是' if test_streaming else '否'}")
        print(f"并行检查: {'是' if parallel else '否'}")
        print("-" * 50)
        
        results = []
        
        if parallel:
            # 并行检查
            with ThreadPoolExecutor(max_workers=5) as executor:
                future_to_backend = {
                    executor.submit(
                        self.check_backend_health, 
                        backend, 
                        model, 
                        timeout, 
                        test_streaming
                    ): backend 
                    for backend in backends
                }
                
                for future in as_completed(future_to_backend):
                    result = future.result()
                    results.append(result)
                    self._print_result(result)
        else:
            # 串行检查
            for backend in backends:
                result = self.check_backend_health(backend, model, timeout, test_streaming)
                results.append(result)
                self._print_result(result)
        
        return results
    
    def _print_result(self, result: Dict):
        """打印单个检查结果"""
        backend = result["backend"]
        status = result["status"]
        response_time = result.get("response_time", 0)
        
        if status == "healthy":
            print(f"✅ {backend}: 健康 ({response_time:.2f}s)")
            if result.get("content"):
                print(f"   响应: {result['content']}")
            elif result.get("first_chunk"):
                print(f"   首块: {result['first_chunk']}")
        elif status == "unhealthy":
            http_status = result.get("http_status", "N/A")
            print(f"❌ {backend}: 不健康 (HTTP {http_status}, {response_time:.2f}s)")
            if result.get("error"):
                print(f"   错误: {result['error']}")
        elif status == "timeout":
            print(f"⏰ {backend}: 超时 ({response_time:.2f}s)")
        else:
            print(f"🔥 {backend}: 错误 ({response_time:.2f}s)")
            if result.get("error"):
                print(f"   错误: {result['error']}")
    
    def continuous_monitoring(
        self, 
        backends: Optional[List[str]] = None,
        model: str = "gpt-4o",
        interval: int = 60,
        timeout: int = 30
    ):
        """持续监控后端健康状态"""
        if backends is None:
            backends = self.get_available_backends()
        
        print(f"🔄 开始持续监控 {len(backends)} 个后端")
        print(f"检查间隔: {interval}s")
        print(f"按 Ctrl+C 停止监控")
        print("=" * 60)
        
        try:
            while True:
                timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                print(f"\n⏰ {timestamp}")
                
                results = self.check_all_backends(
                    backends, model, timeout, parallel=True
                )
                
                # 统计
                healthy_count = sum(1 for r in results if r["status"] == "healthy")
                total_count = len(results)
                
                print(f"\n📊 统计: {healthy_count}/{total_count} 个后端健康")
                
                if healthy_count < total_count:
                    unhealthy = [r["backend"] for r in results if r["status"] != "healthy"]
                    print(f"🚨 不健康的后端: {', '.join(unhealthy)}")
                
                print(f"\n💤 等待 {interval} 秒...")
                time.sleep(interval)
                
        except KeyboardInterrupt:
            print("\n\n👋 监控已停止")
    
    def benchmark_backends(
        self, 
        backends: Optional[List[str]] = None,
        model: str = "gpt-4o",
        rounds: int = 3
    ):
        """对后端进行性能基准测试"""
        if backends is None:
            backends = self.get_available_backends()
        
        print(f"🏃 对 {len(backends)} 个后端进行性能测试")
        print(f"测试轮数: {rounds}")
        print("=" * 50)
        
        all_results = {}
        
        for round_num in range(1, rounds + 1):
            print(f"\n🔄 第 {round_num} 轮测试")
            print("-" * 30)
            
            results = self.check_all_backends(
                backends, model, timeout=30, parallel=False
            )
            
            for result in results:
                backend = result["backend"]
                if backend not in all_results:
                    all_results[backend] = []
                all_results[backend].append(result)
            
            if round_num < rounds:
                print("⏳ 等待 5 秒...")
                time.sleep(5)
        
        # 计算统计信息
        print(f"\n📊 性能统计 ({rounds} 轮)")
        print("=" * 50)
        
        for backend in sorted(all_results.keys()):
            results = all_results[backend]
            healthy_results = [r for r in results if r["status"] == "healthy"]
            
            if healthy_results:
                response_times = [r["response_time"] for r in healthy_results]
                avg_time = sum(response_times) / len(response_times)
                min_time = min(response_times)
                max_time = max(response_times)
                success_rate = len(healthy_results) / len(results) * 100
                
                print(f"✅ {backend}:")
                print(f"   成功率: {success_rate:.1f}%")
                print(f"   平均响应时间: {avg_time:.2f}s")
                print(f"   最快响应: {min_time:.2f}s")
                print(f"   最慢响应: {max_time:.2f}s")
            else:
                print(f"❌ {backend}: 所有测试都失败")
            print()

def main():
    parser = argparse.ArgumentParser(description="后端健康检查工具")
    parser.add_argument("--url", default="http://localhost:3000", help="API基础URL")
    parser.add_argument("--token", default="test-token", help="认证令牌")
    parser.add_argument("--model", default="gpt-4o", help="测试模型")
    parser.add_argument("--timeout", type=int, default=30, help="请求超时时间(秒)")
    parser.add_argument("--backends", nargs="+", help="指定要测试的后端")
    
    subparsers = parser.add_subparsers(dest="command", help="可用命令")
    
    # 单次检查
    check_parser = subparsers.add_parser("check", help="单次健康检查")
    check_parser.add_argument("--streaming", action="store_true", help="测试流式请求")
    check_parser.add_argument("--serial", action="store_true", help="串行检查（非并行）")
    
    # 持续监控
    monitor_parser = subparsers.add_parser("monitor", help="持续监控")
    monitor_parser.add_argument("--interval", type=int, default=60, help="检查间隔(秒)")
    
    # 性能测试
    benchmark_parser = subparsers.add_parser("benchmark", help="性能基准测试")
    benchmark_parser.add_argument("--rounds", type=int, default=3, help="测试轮数")
    
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        return
    
    checker = BackendHealthChecker(args.url, args.token)
    
    if args.command == "check":
        checker.check_all_backends(
            backends=args.backends,
            model=args.model,
            timeout=args.timeout,
            test_streaming=args.streaming,
            parallel=not args.serial
        )
    elif args.command == "monitor":
        checker.continuous_monitoring(
            backends=args.backends,
            model=args.model,
            interval=args.interval,
            timeout=args.timeout
        )
    elif args.command == "benchmark":
        checker.benchmark_backends(
            backends=args.backends,
            model=args.model,
            rounds=args.rounds
        )

if __name__ == "__main__":
    main()
