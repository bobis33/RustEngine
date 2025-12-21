#!/bin/bash
set -e

VERSION=1.4.341.1
PLATFORM=linux
ARCHIVE=vulkansdk-$PLATFORM-x86_64-$VERSION.tar.xz
INSTALL_DIR=$HOME/VulkanSDK/$VERSION

echo "==> Downloading Vulkan SDK $VERSION for $PLATFORM..."
curl -L -o $ARCHIVE https://sdk.lunarg.com/sdk/download/$VERSION/$PLATFORM/$ARCHIVE

echo "==> Extracting..."
tar xf $ARCHIVE

# The archive extracts directly as $VERSION/ at the root, no VulkanSDK/ parent
echo "==> Installing to $INSTALL_DIR..."
mkdir -p $HOME/VulkanSDK
mv $VERSION $HOME/VulkanSDK/

echo "==> Cleaning up..."
rm -f $ARCHIVE

echo ""
echo "==> Done! Add the following lines to your ~/.bashrc or ~/.zshrc:"
echo ""
echo "export VULKAN_SDK=$INSTALL_DIR/x86_64"
echo "export PATH=\$VULKAN_SDK/bin:\$PATH"
echo "export LD_LIBRARY_PATH=\$VULKAN_SDK/lib:\$LD_LIBRARY_PATH"
echo "export VK_ICD_FILENAMES=\$VULKAN_SDK/etc/vulkan/icd.d"
echo "export VK_LAYER_PATH=\$VULKAN_SDK/etc/vulkan/explicit_layer.d"