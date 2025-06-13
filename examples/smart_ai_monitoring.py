#!/usr/bin/env python3
"""
SmartAI 监控示例脚本
用于监控SmartAI负载均衡器的权重分布和健康状态
"""

import requests
import json
import time
from datetime import datetime
from typing import Dict, List, Any

class SmartAIMonitor:
    def __init__(self, base_url: str = "http://localhost:3000"):
        self.base_url = base_url
        
    def get_smart_ai_weights(self, detailed: bool = False, enabled_only: bool = True) -> Dict[str, Any]:
        """获取所有SmartAI模型的权重信息"""
        params = {
            "detailed": str(detailed).lower(),
            "enabled_only": str(enabled_only).lower()
        }
        
        try:
            response = requests.get(f"{self.base_url}/smart-ai/weights", params=params)
            response.raise_for_status()
            return response.json()
        except requests.RequestException as e:
            print(f"❌ 获取权重信息失败: {e}")
            return {}
    
    def get_model_weights(self, model_name: str, detailed: bool = False) -> Dict[str, Any]:
        """获取特定模型的权重信息"""
        params = {"detailed": str(detailed).lower()}
        
        try:
            response = requests.get(f"{self.base_url}/smart-ai/models/{model_name}/weights", params=params)
            response.raise_for_status()
            return response.json()
        except requests.RequestException as e:
            print(f"❌ 获取模型 {model_name} 权重信息失败: {e}")
            return {}
    
    def analyze_cost_distribution(self) -> None:
        """分析成本分布"""
        print("💰 成本分布分析")
        print("=" * 50)
        
        data = self.get_smart_ai_weights()
        if not data or "models" not in data:
            print("❌ 无法获取数据")
            return
        
        for model in data["models"]:
            print(f"\n📊 模型: {model['name']}")
            
            total_weight = 0
            premium_weight = 0
            
            for backend in model["backends"]:
                if backend["enabled"]:
                    weight = backend["effective_weight"]
                    total_weight += weight
                    
                    if backend["is_premium"]:
                        premium_weight += weight
                        status = "💎 Premium"
                    else:
                        status = "💚 便宜"
                    
                    print(f"  {status} {backend['provider']}: {weight:.3f} (信心度: {backend['confidence']:.3f})")
            
            if total_weight > 0:
                premium_ratio = premium_weight / total_weight * 100
                print(f"  📈 Premium后端权重占比: {premium_ratio:.1f}%")
                
                if premium_ratio > 50:
                    print("  ⚠️  警告: Premium后端权重过高，可能增加成本")
                elif premium_ratio < 20:
                    print("  ✅ 良好: 主要使用便宜后端")
                else:
                    print("  ℹ️  正常: 权重分布合理")
    
    def check_health_status(self) -> None:
        """检查健康状态"""
        print("\n🏥 健康状态检查")
        print("=" * 50)
        
        data = self.get_smart_ai_weights(detailed=True)
        if not data or "models" not in data:
            print("❌ 无法获取数据")
            return
        
        for model in data["models"]:
            print(f"\n📊 模型: {model['name']}")
            stats = model["stats"]
            
            print(f"  总后端: {stats['total_backends']}")
            print(f"  健康后端: {stats['healthy_backends']}")
            print(f"  平均信心度: {stats['average_confidence']:.3f}")
            
            health_ratio = stats['healthy_backends'] / stats['total_backends'] * 100 if stats['total_backends'] > 0 else 0
            
            if health_ratio >= 80:
                print(f"  ✅ 健康状态良好 ({health_ratio:.1f}%)")
            elif health_ratio >= 50:
                print(f"  ⚠️  健康状态一般 ({health_ratio:.1f}%)")
            else:
                print(f"  ❌ 健康状态较差 ({health_ratio:.1f}%)")
            
            # 检查问题后端
            problem_backends = []
            for backend in model["backends"]:
                if backend["confidence"] < 0.6:
                    problem_backends.append(f"{backend['provider']} (信心度: {backend['confidence']:.3f})")
            
            if problem_backends:
                print(f"  🚨 问题后端: {', '.join(problem_backends)}")
    
    def monitor_weights_change(self, interval: int = 30, duration: int = 300) -> None:
        """监控权重变化"""
        print(f"📈 开始监控权重变化 (间隔: {interval}秒, 持续: {duration}秒)")
        print("=" * 60)
        
        start_time = time.time()
        previous_weights = {}
        
        while time.time() - start_time < duration:
            current_time = datetime.now().strftime("%H:%M:%S")
            print(f"\n⏰ {current_time}")
            
            data = self.get_smart_ai_weights()
            if not data or "models" not in data:
                print("❌ 无法获取数据")
                time.sleep(interval)
                continue
            
            current_weights = {}
            
            for model in data["models"]:
                model_name = model["name"]
                current_weights[model_name] = {}
                
                for backend in model["backends"]:
                    if backend["enabled"]:
                        backend_key = f"{backend['provider']}:{backend['model']}"
                        weight = backend["effective_weight"]
                        confidence = backend["confidence"]
                        
                        current_weights[model_name][backend_key] = {
                            "weight": weight,
                            "confidence": confidence
                        }
                        
                        # 检查权重变化
                        if model_name in previous_weights and backend_key in previous_weights[model_name]:
                            prev_weight = previous_weights[model_name][backend_key]["weight"]
                            weight_change = weight - prev_weight
                            
                            if abs(weight_change) > 0.01:  # 权重变化超过0.01
                                change_symbol = "📈" if weight_change > 0 else "📉"
                                print(f"  {change_symbol} {model_name} - {backend_key}: {prev_weight:.3f} → {weight:.3f} (Δ{weight_change:+.3f})")
            
            previous_weights = current_weights
            time.sleep(interval)
        
        print("\n✅ 监控结束")
    
    def generate_report(self) -> None:
        """生成综合报告"""
        print("📋 SmartAI 综合报告")
        print("=" * 60)
        print(f"生成时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        
        self.analyze_cost_distribution()
        self.check_health_status()
        
        # 获取详细数据进行更深入分析
        data = self.get_smart_ai_weights(detailed=True)
        if data and "models" in data:
            print(f"\n📊 总体统计")
            print("=" * 30)
            print(f"SmartAI模型总数: {data['total_smart_ai_models']}")
            
            total_backends = sum(model["stats"]["total_backends"] for model in data["models"])
            total_healthy = sum(model["stats"]["healthy_backends"] for model in data["models"])
            total_premium = sum(model["stats"]["premium_backends"] for model in data["models"])
            
            print(f"总后端数: {total_backends}")
            print(f"健康后端数: {total_healthy}")
            print(f"Premium后端数: {total_premium}")
            
            if total_backends > 0:
                print(f"整体健康率: {total_healthy/total_backends*100:.1f}%")
                print(f"Premium后端比例: {total_premium/total_backends*100:.1f}%")

def main():
    monitor = SmartAIMonitor()
    
    print("🤖 SmartAI 监控工具")
    print("=" * 40)
    
    while True:
        print("\n请选择操作:")
        print("1. 成本分布分析")
        print("2. 健康状态检查") 
        print("3. 权重变化监控")
        print("4. 生成综合报告")
        print("5. 退出")
        
        choice = input("\n请输入选项 (1-5): ").strip()
        
        if choice == "1":
            monitor.analyze_cost_distribution()
        elif choice == "2":
            monitor.check_health_status()
        elif choice == "3":
            try:
                interval = int(input("监控间隔(秒, 默认30): ") or "30")
                duration = int(input("监控时长(秒, 默认300): ") or "300")
                monitor.monitor_weights_change(interval, duration)
            except ValueError:
                print("❌ 输入无效，使用默认值")
                monitor.monitor_weights_change()
        elif choice == "4":
            monitor.generate_report()
        elif choice == "5":
            print("👋 再见!")
            break
        else:
            print("❌ 无效选项，请重新选择")

if __name__ == "__main__":
    main()
