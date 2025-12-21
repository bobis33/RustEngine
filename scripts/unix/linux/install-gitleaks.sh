#!/bin/bash
set -e

version=8.30.0
platform=linux

wget https://github.com/gitleaks/gitleaks/releases/download/v$version/gitleaks_$version_$platform_x64.tar.gz
tar -xf gitleaks_$version_$platform_x64.tar.gz
sudo mv gitleaks /usr/local/bin/
