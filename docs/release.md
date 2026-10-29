# 发布规则

> 上级：[实现方案索引](impl.md) ｜ 相关：[测试与交付](impl/quality.md) ｜ [决策记录](decisions.md)
>
> 本文拥有以下规则的完整定义：Release 资产命名、tag 约定、上传与改名流程、未来 CI 矩阵。`quality.md` 只保留摘要并链接到本文。

## 1. 资产命名

一个发布产物 = 一个平台 + 一个架构，文件名必须自解释。

| 平台 | `os` | 文件名 |
|---|---|---|
| Windows x86_64 | `windows` | `lsm-windows-x64.exe` |
| Linux x86_64 | `linux` | `lsm-linux-x64` |
| macOS ARM64 | `macos` | `lsm-macos-arm64` |

规则：

1. 格式固定为 `lsm-<os>-<arch>`，Windows 追加 `.exe`，Unix 平台不带扩展名。
2. `os` 只能是 `windows` / `linux` / `macos`；`arch` 只能是 `x64` / `arm64`。
3. **版本号不进文件名**：版本由 Release tag 表达，重复写进文件名只会让下载名变长。
4. **同一 Release 内资产名必须唯一**：GitHub 以 `name` 作为唯一键，重名上传会被拒绝（422）。
5. **不使用 `label`**：平台与架构只体现在 `name` 上。`label` 仅影响页面展示，既不进下载 URL，也解决不了重名。
6. 因此禁止「三个平台都叫 `lsm.exe`，只用 label 区分」：同 Release 下无法共存，用户下载后也会互相覆盖。

正确与错误示例：

```
✅ lsm-windows-x64.exe   lsm-linux-x64   lsm-macos-arm64
❌ lsm.exe（三平台同名）           ❌ lsm-v0.1.1-windows-x86_64.exe（版本冗余、名字过长）
```

## 2. Tag 约定

- 格式 `vMAJOR.MINOR.PATCH`，与 `Cargo.toml` 的 `version` 一致。
- 用 annotated tag：`git tag -a v0.1.1 -m "v0.1.1: <一句话说明>"`。
- 版本号只改 `Cargo.toml`（`Cargo.lock` 随之更新），文档里的版本示例同一次提交内同步。
- 新 Release 使用 `--latest`；旧 Release 会自动变为非 Latest，不删除。

## 3. 发布流程

```powershell
# 1. 版本与测试
#    改 Cargo.toml 的 version，然后：
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test

# 2. 提交与打 tag
git commit -am "chore: bump version to X.Y.Z"
git tag -a vX.Y.Z -m "vX.Y.Z: <一句话说明>"

# 3. 构建并把产物改成规范名
cargo build --release
Copy-Item .\target\release\lsm.exe .\target\release\lsm-windows-x64.exe -Force
.\target\release\lsm-windows-x64.exe --version   # 确认版本号

# 4. 推送并发布（资产名 = 规范名，不带 label）
git push origin main
git push origin vX.Y.Z
gh release create vX.Y.Z .\target\release\lsm-windows-x64.exe `
  --title "vX.Y.Z" --generate-notes --latest
```

校验（必做）：

```powershell
gh release view vX.Y.Z --json assets --jq '.assets[] | {name, size, state}'
# 下载一次确认可达，字节数与本地一致
curl.exe -s -o NUL -w "%{http_code} %{size_download}`n" -L `
  "https://github.com/yearzen1/LocalSkillManager/releases/download/vX.Y.Z/lsm-windows-x64.exe"
```

## 4. 资产改名

已发布资产的命名不合规时，用 API 改名（内容不变、不重新上传）：

```powershell
# 取资产 id
gh api repos/yearzen1/LocalSkillManager/releases --jq '.[] | {tag: .tag_name, assets: [.assets[] | {id, name}]}'

# 改名（只改文件名；历史资产若带 label，一并清空）
'{"name": "lsm-windows-x64.exe", "label": ""}' | Set-Content .\asset.json -Encoding utf8 -NoNewline
gh api -X PATCH repos/yearzen1/LocalSkillManager/releases/assets/<id> --input .\asset.json
Remove-Item .\asset.json
```

只传 `name` 就能改名；`label` 为可选展示字段，本项目一律留空。清空已有 label 必须显式传 `"label": ""`，省略该字段会保持原值。

改名后必须重新校验下载 URL——旧链接会失效，这属于可接受的代价，但要在改名时确认没有外部依赖旧链接。

## 5. 未来 CI 矩阵

- 第二阶段的 GitHub Actions 产物**直接产出规范文件名**，不再事后改名。
- 矩阵：`windows-latest` / `ubuntu-latest` / `macos-latest`；首期只出 `x64`，`arm64` 作为增量。
- 压缩包仅在需要时使用，且仍以平台命名：`lsm-linux-x64.tar.gz`；Windows 直接给 `.exe`，不额外打包。
- CI 与手工发布的文件名规则必须一致，便于脚本替换。

## 6. 相关决策

- Q6 / Q13：v1 只做本地构建，CI 后置。
- Q23：MIT 许可证。

完整决策见 [decisions.md](decisions.md)。

