#!/bin/bash

# 更新berry-loadbalance中的导入路径
echo "Updating imports in berry-loadbalance..."

# 更新config相关导入
find berry-loadbalance/src -name "*.rs" -exec sed -i '' 's/use crate::config::/use berry_core::config::/g' {} \;
find berry-loadbalance/src -name "*.rs" -exec sed -i '' 's/crate::config::/berry_core::config::/g' {} \;

# 更新berry-relay中的导入路径
echo "Updating imports in berry-relay..."

find berry-relay/src -name "*.rs" -exec sed -i '' 's/use crate::config::/use berry_core::config::/g' {} \;
find berry-relay/src -name "*.rs" -exec sed -i '' 's/crate::config::/berry_core::config::/g' {} \;
find berry-relay/src -name "*.rs" -exec sed -i '' 's/use crate::loadbalance::/use berry_loadbalance::/g' {} \;
find berry-relay/src -name "*.rs" -exec sed -i '' 's/crate::loadbalance::/berry_loadbalance::/g' {} \;
find berry-relay/src -name "*.rs" -exec sed -i '' 's/use crate::auth::/use berry_core::auth::/g' {} \;
find berry-relay/src -name "*.rs" -exec sed -i '' 's/crate::auth::/berry_core::auth::/g' {} \;

# 更新berry-api中的导入路径
echo "Updating imports in berry-api..."

find berry-api/src -name "*.rs" -exec sed -i '' 's/use crate::config::/use berry_core::config::/g' {} \;
find berry-api/src -name "*.rs" -exec sed -i '' 's/crate::config::/berry_core::config::/g' {} \;
find berry-api/src -name "*.rs" -exec sed -i '' 's/use crate::loadbalance::/use berry_loadbalance::/g' {} \;
find berry-api/src -name "*.rs" -exec sed -i '' 's/crate::loadbalance::/berry_loadbalance::/g' {} \;
find berry-api/src -name "*.rs" -exec sed -i '' 's/use crate::auth::/use berry_core::auth::/g' {} \;
find berry-api/src -name "*.rs" -exec sed -i '' 's/crate::auth::/berry_core::auth::/g' {} \;
find berry-api/src -name "*.rs" -exec sed -i '' 's/use crate::relay::/use berry_relay::/g' {} \;
find berry-api/src -name "*.rs" -exec sed -i '' 's/crate::relay::/berry_relay::/g' {} \;

echo "Import updates completed!"
