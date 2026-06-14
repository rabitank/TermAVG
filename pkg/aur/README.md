# AUR 发布

## 首次

注册 AUR 账号、添加 SSH key 后：

```bash
git clone ssh://aur@aur.archlinux.org/tmj-bin.git
git clone ssh://aur@aur.archlinux.org/tmj-wgpu-bin.git
```

## 每次 release

假设新版本为 `0.2.0`，GitHub Release 已发布：

```bash
V=0.2.0

# ---- tmj-bin ----
cd tmj-bin
cp ../TerminalLove/engine/pkg/aur/tmj-bin/PKGBUILD .
sed -i "s/^pkgver=.*/pkgver=$V/" PKGBUILD
sed -i "s/^pkgrel=.*/pkgrel=1/" PKGBUILD
# 可选：计算实际 sha256sum
# sha256sum_x86_64=("$(curl -sL ... | sha256sum | cut -d' ' -f1)")
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "v$V"
git push
cd ..

# ---- tmj-wgpu-bin ----
cd tmj-wgpu-bin
cp ../TerminalLove/engine/pkg/aur/tmj-wgpu-bin/PKGBUILD .
sed -i "s/^pkgver=.*/pkgver=$V/" PKGBUILD
sed -i "s/^pkgrel=.*/pkgrel=1/" PKGBUILD
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "v$V"
git push
cd ..
```

## Arch 用户安装

```bash
yay -S tmj-bin           # 终端模式
yay -S tmj-wgpu-bin      # GPU 窗口模式
```
