# 自更新（upgrade）

> 上级：[实现方案索引](../impl.md) ｜ 相关：[CLI 与输出](cli.md) ｜ [测试与交付](quality.md) ｜ [发布规则](../release.md) ｜ [决策记录](../decisions.md)
>
> 本文拥有以下规则的完整定义：`upgrade` 命令、版本比较、平台资产选择、下载与校验、原地替换与回滚、升级失败的行为。其他文档只做摘要和链接。
>
> 状态：规则已定，代码未实现（见 [impl.md](../impl.md)「实现顺序」第 13–18 片）。

## 1. 命令与行为

```
lsm upgrade [--check] [--force]
```

| 输入 | 行为 | 退出码 |
|---|---|---|
| `upgrade` | 查最新 release，比版本；有新版本则下载、校验、原地替换 | 0（替换成功或已最新）／1（任一环节失败） |
| `upgrade --check` | 只查询并报告，不下载、不写盘 | 0（发现新版本也是 0；查询失败为 1，见第 7 节） |
| `upgrade --force` | 同版本也重装；latest 低于当前版本时执行降级 | 同上 |
| `upgrade --check --force` | `--force` 对 `--check` 无意义，不报错，忽略 | 同上 |

- 版本关系判定：
  - latest > 当前：安装（`--check` 只报告）。
  - latest == 当前：`--force` 才重装，否则打印已是最新后退出 0。
  - latest < 当前：打印当前版本更新；`--force` 才安装（等于回滚到该 release）。
- 升级只动当前可执行文件，**不读写 `.agents/skills`**，也不检查 skill 是否需要更新。
- 不做自动检查、不做后台检查、不做启动时提示：只有显式执行 `upgrade` 才联网。
- 不加短选项：`upgrade` 不参与 `cli::normalize` 的 `-a/-l/-c` 归一化，`lsm -u` 按未知参数退出 2。

## 2. 版本与资产

### 2.1 版本格式

- release tag 唯一合法形式：`vMAJOR.MINOR.PATCH`，三段都是十进制数字，不允许前导 `+`/`-`、预发布后缀或 build metadata。
- 解析时也接受不带 `v` 的 `MAJOR.MINOR.PATCH`（`CARGO_PKG_VERSION` 就是这种形式）；内部只保存三段数字，比较时按数值。输出 release 版本时统一带 `v` 前缀，输出当前版本时与 `CARGO_PKG_VERSION` 一致。
- 不合法的 tag（如 `v0.2`、`v0.2.0-rc.1`、`latest`）直接报错，不猜测、不截断。
- 比较按 `(major, minor, patch)` 数值元组逐段比较，前导零按数值处理（`v0.02.0` 视为 `0.2.0`）。
- 当前版本取自编译期 `CARGO_PKG_VERSION`，不读文件、不执行外部命令。
- 不引入 `semver` 依赖：解析与比较各约十行，代码在 `domain::version`。

### 2.2 平台与资产名

资产名是升级的唯一查找键，由 `os` 与 `arch` 拼成：

| 运行平台 | `os` | `arch` | 资产名 |
|---|---|---|---|
| Windows x86_64 | `windows` | `x64` | `lsm-windows-x64.exe` |
| Windows ARM64 | `windows` | `arm64` | `lsm-windows-arm64.exe` |
| Linux x86_64 | `linux` | `x64` | `lsm-linux-x64` |
| Linux ARM64 | `linux` | `arm64` | `lsm-linux-arm64` |
| macOS ARM64 | `macos` | `arm64` | `lsm-macos-arm64` |
| macOS x86_64 | `macos` | `x64` | `lsm-macos-x64` |

- `os` 取 `std::env::consts::OS`（`windows` / `linux` / `macos`），`arch` 取 `std::env::consts::ARCH`（`x86_64` → `x64`，`aarch64` → `arm64`）。
- 表里列的是命名规则全集；当前实际发布只覆盖 `lsm-windows-x64.exe`、`lsm-linux-x64`、`lsm-macos-arm64`（见 [release.md](../release.md)「资产命名」），其余组合即使命名规则成立也会报「release 里没有当前平台资产」。
- 匹配是**精确相等**，不按前缀、不按 `contains`、不按 `label`，也不接受压缩包（`.zip` / `.tar.gz`）。
- release 里没有匹配资产时，错误信息必须带上期望的资产名与 release tag；未知 OS/arch 同样按资产缺失处理，并提示手动下载。

### 2.3 release 元数据

只读取以下字段（其余忽略，未知字段不影响解析）：

```json
{
  "tag_name": "v0.2.0",
  "assets": [
    {
      "name": "lsm-windows-x64.exe",
      "browser_download_url": "https://github.com/.../lsm-windows-x64.exe"
    }
  ]
}
```

- 下载地址取 `browser_download_url`，不自行拼接 URL（私有镜像与 GitHub 的路径规则不同）。
- 同一资产名出现多次时取第一个，视为发布端错误但不阻断升级。
- 缺少 `tag_name`、`assets` 不是数组、字段类型不对时，按「响应无法解析」报错，不做容错猜测。

## 3. 网络

- 默认端点：`https://api.github.com/repos/yearzen1/LocalSkillManager/releases/latest`。
- 唯一覆盖入口：环境变量 `LSM_UPGRADE_API`（空串按未设置处理），必须是 `http://` 或 `https://` 的绝对 URL，指向与上面同结构的 JSON。用途：GitHub 不可达时的镜像、以及测试桩。
- 请求头固定：`User-Agent: lsm/<当前版本>`（GitHub API 要求）、`Accept: application/vnd.github+json`。
- 重定向（含 GitHub 资产跳转到 `objects.githubusercontent.com`）由 HTTP 客户端自动跟随，上限 10 次，超过即失败。
- 超时：元数据请求 30 秒；资产下载 5 分钟。超时按网络失败处理，不重试。
- 不读 `GITHUB_TOKEN`、不支持 `--token`、不做代理参数；HTTP 403 按限流处理，提示稍后重试或手动下载。
- 网络实现只在 `infra::net`：`commands` 不直接调用 HTTP 客户端，`cli` 不访问网络。
- HTTP 客户端固定为 `ureq` 3（`default-features = false, features = ["rustls"]`）：阻塞式、无 async 运行时，rustls + ring + 内置 webpki 根证书；`max_redirects` 用库默认的 10，不额外配置；元数据与资产各用一个 Agent，`timeout_global` 分别为 30 秒与 5 分钟；403 由 `http_status_as_error` 直接映射为限流提示；资产用 `body.as_reader()` 流式写盘，不整包读进内存。根证书是内置列表、不随系统更新，靠升级 ureq 版本刷新；内网 MITM 代理场景用 `LSM_UPGRADE_API` 镜像兜底（Q36）。
- 默认端点必须走 HTTPS；`LSM_UPGRADE_API` 显式写成 `http://` 时按用户选择执行（测试桩、内网镜像）。除 TLS 之外不做额外来源校验。

## 4. 下载与校验

1. 下载目标：`<exe 所在目录>/.lsm-upgrade-<pid>`，与 exe 同卷，保证后续 `rename` 原子。
2. 落盘前先清理同目录遗留的 `.lsm-upgrade-*`（并发升级不受支持，见第 6 节）。
3. Unix 上先按原 exe 的权限位设置新文件（读不到原权限时用 `0o755`），否则无法执行冒烟校验；Windows 不处理 ACL。
4. **冒烟校验**（`--check` 不做）：执行 `<新文件> --version`，要求：
   - 进程退出码 0；
   - stdout 中出现 `lsm <tag 去掉 v>`（与 release 的 `tag_name` 一致）。
   任一不满足即判定下载内容不可用，删除临时文件，原 exe 不动。
5. release 资产目前没有 checksum 文件，因此 v1 **不做**哈希或签名校验；实际防线是 HTTPS + 上面的冒烟校验。发布端将来补 `.sha256` 时再单独立决策。
6. 校验通过才进入替换步骤；`--check` 在第 2.1 节的版本比较后就结束，不下载。

## 5. 原地替换与回滚

**硬规则：升级后 exe 的路径与文件名不变。** 不生成 `lsm-v0.2.0.exe`、不把 `lsm.exe` 改成 `lsm-old.exe` 长期保留。用户把二进制改名成 `myskillmgr.exe` 后升级，结果仍是同一个 `myskillmgr.exe`。

- 目标路径取 `current_exe()`，不按 `lsm` 这个名字查找；路径是符号链接时先 `canonicalize`，替换链接指向的真实文件，**链接本身保持原样**。
- Unix：`rename(临时文件, exe)` 直接覆盖，一步原子完成。
- Windows（运行中的 exe 不能被覆盖，但可以被改名）：
  1. `rename(exe, <exe>.old-<pid>)`；
  2. `rename(临时文件, exe)`；
  3. 删除 `<exe>.old-<pid>`。
  第 2 步失败时把第 1 步改回原名；第 3 步失败只打 warning（Windows 上旧映像可能仍被占用），退出码仍为 0，下次升级开始时清理同目录 `<exe>.old-*`。
- 目录不可写（如 `/usr/local/bin`）：报错并给出路径与 hint（用有权限的账户重跑，或从 release 页手动下载替换），临时文件清理干净。
- 当前 exe 位于 `$CARGO_HOME/bin`（默认 `~/.cargo/bin`）时，打一条 warning 提示 `cargo install --force local-skill-manager` 才是可持续的更新方式，但**不阻止**本次替换。
- 升级期间的残留只允许出现在 exe 同目录，且必须是点开头的 `.lsm-upgrade-*` 或 `<exe>.old-*`；任何情况下不得留下改名后的可执行文件。
- 并发升级不做锁：两个进程同时替换时，以其中一个 `rename` 失败（IO 错误）结束；`rename` 的原子性保证 exe 始终是某个完整版本。

## 6. 安全与失败边界

- 只在用户显式执行 `upgrade` 时联网；不新增后台进程、不写启动项、不读配置文件。
- 失败时的唯一要求：原 exe 仍可执行；临时文件与备份不残留（Windows 删不掉的备份除外，见第 5 节）。
- 不做特权提升：需要写权限时只报错，不调用 `sudo`、不弹 UAC。
- 不自动重试、不自动回滚到上一版本；回滚方式是 `upgrade --force` 装回目标 release。
- 升级不校验 release 的发布时间、作者或签名，也不比对资产大小。

## 7. 输出与退出码

成功与提示走 stdout，warning 与错误走 stderr，全部来自 `i18n` 消息表（en/zh 占位符一致）。

| 场景 | 输出（en） | 退出码 |
|---|---|---|
| 已是最新 | `lsm 0.1.1 is up to date` | 0 |
| `--check` 有新版本 | `new version available: v0.2.0 (current 0.1.1)` | 0 |
| 安装成功 | `upgraded lsm 0.1.1 -> v0.2.0 (<exe 路径>)` | 0 |
| 降级（`--force`，latest 更旧） | `downgraded lsm 0.2.0 -> v0.1.1 (<exe 路径>)` | 0 |
| 当前版本更新且未加 `--force` | `lsm 0.2.0 is newer than v0.1.1` | 0 |
| 当前平台无资产 | `error: release v0.2.0 has no asset for <os>-<arch>` + hint `expected lsm-<os>-<arch>` | 1 |
| 元数据请求失败（网络/非 200/解析） | `error: cannot fetch latest release: <原因>` + hint 镜像与手动下载 | 1 |
| 限流（HTTP 403/429） | `error: github api rate limit reached` + hint 稍后重试或手动下载 | 1 |
| 下载失败 | `error: cannot download <资产名>: <原因>` + hint 手动下载 | 1 |
| 冒烟校验失败 | `error: downloaded binary failed verification` + hint 检查 release 资产 | 1 |
| 替换失败/目录不可写 | `error: cannot replace <exe 路径>: <原因>` + hint 权限与手动下载 | 1 |
| 备份删除失败 | `warning: old binary left at <exe>.old-<pid>, it will be cleaned on next upgrade` | 0 |
| cargo 安装场景 | `warning: lsm is installed by cargo, consider cargo install --force local-skill-manager` | 0（继续替换） |

- `--check` 从不写盘，也从不输出安装成功文案。
- stdout 在成功路径上只有一行结果；不做进度条、不做百分比、不做 `--json`。

## 8. 测试策略

单测与集成测试的分工：

- `domain::version`：合法/非法 tag、前导 `v`、三段比较、前导零、降级判断。
- `domain::release`：资产选择（命中/缺失/同名重复/压缩包不匹配）、平台映射（三平台 + 未知组合）。
- `infra::net`：用 `std::net::TcpListener` 起本地桩，覆盖 200、302 跳转、404、403、非 JSON、超时；断言请求头含 `User-Agent`。
- `infra::fsops`（自更新部分）：临时目录内替换成功、权限位保留、备份清理、回滚（新文件缺失时旧文件仍可执行）。
- `commands::upgrade`：已最新、有新版本、`--check` 不落盘、`--force` 同版本重装与降级、不可写目录报错。
- 集成测试（`tests/cli.rs`）：
  - 把已构建的 `lsm` 二进制复制到临时目录后运行该副本，`current_exe()` 自然指向副本，**绝不替换 `target/` 下的真实构建产物**；
  - 用 `LSM_UPGRADE_API` 指向本地桩，桩同时提供元数据与二进制（载荷为同一二进制追加 1 字节的副本）；
  - 断言：`--check` 只打印不写盘；`upgrade --force` 后路径与文件名不变、内容长度等于载荷、`.old-*` 已清理；
  - 覆盖资产缺失、404、限流、未知参数、`--help` 两种语言。
- 不做真实 GitHub 的联网测试；发布前手工验收见 [quality.md](quality.md)「手工验收」。

## 9. 明确不做

- 不做 checksum / 签名校验（等发布端产出 `.sha256` 再立决策）。
- 不读 `GITHUB_TOKEN`、不支持 `--token`、不做代理参数。
- 不做自动/后台检查更新，不做启动时版本提示。
- 不做 `--json`、进度条、`--version <tag>` 指定版本；回滚用 `upgrade --force`。
- 不更新通过包管理器安装的副本（apt、brew、scoop 等），遇到只报权限错误或提示手动处理。
- 不改 `Cargo.toml` 版本号：`upgrade` 只替换二进制，版本 bump 仍由发布流程负责。

## 10. 相关决策

- Q30：自更新范围与命令形态。
- Q31：校验与原地替换（文件名不变）。
- Q32：网络端点、`LSM_UPGRADE_API`、资产名契约。

完整决策见 [decisions.md](../decisions.md)。
