# seu_wlan_login

东南大学校园网 WLAN 自动登录命令行工具。

## 特性

- 三种凭据获取模式：命令行参数、环境变量、交互式输入
- 密码交互式输入时不回显（基于 `rpassword`）
- 自动查询当前分配的内网 IP 并完成认证
- 详细的错误提示（账号不存在、密码错误、已登录等）
- `--debug` 模式打印完整的登录后状态信息

## 安装

```bash
cargo build --release
```

生成的可执行文件位于 `target/release/seu_wlan_login`，可复制到 `PATH` 中使用。

## 用法

```
seu_wlan_login [OPTIONS]
```

### 选项

| 选项                          | 说明                                      |
|-----------------------------|-----------------------------------------|
| `-m, --mode <MODE>`         | 凭据获取模式：`cli` / `env` / `inter`，默认 `env` |
| `-a, --account <ACCOUNT>`   | 一卡通账号（`cli` 模式必填）                       |
| `-p, --password <PASSWORD>` | 密码（`cli` 模式必填）                          |
| `-d, --debug`               | 登录成功后打印完整状态信息                           |

### 凭据获取模式

**cli 模式**：通过命令行参数提供账号密码

```bash
seu_wlan_login --mode cli --account 213123456 --password mypassword
```

**env 模式**（默认）：通过环境变量提供账号密码

```bash
export SEU_WLAN_ACCOUNT=213123456
export SEU_WLAN_PASSWORD=mypassword
seu_wlan_login
```

**inter 模式**：运行时交互式输入，密码输入不回显

```bash
seu_wlan_login --mode inter
```

### 调试模式

```bash
seu_wlan_login --mode inter --debug
```

输出示例（部分字段）：

```text
Login successful!
WlanStatus {
    result: 1,
    v46ip: "10.0.0.1",
    time: Some(1234),
    flow: Some(5678),
    ...
}
```

## 工作原理

1. 请求 `https://w.seu.edu.cn/drcom/chkstatus?callback=dr1002` 查询当前认证状态并获取内网 IPv4 地址。
2. 若已登录（`result == 1`）则直接报错退出。
3. 请求 `https://w.seu.edu.cn:801/eportal/`，携带账号、密码、内网 IP 等参数发起认证。
4. 若失败，将返回信息 Base64 解码后映射为具体错误；若成功，再次查询状态并返回。

## 注意事项

1. 请关闭所有代理和VPN，如在内网却无法访问 `https://w.seu.edu.cn`，大概率是使用了代理。
2. 如果遇到奇怪的解码问题，请使用 `--debug` flag 输出每一步的调试信息进行排查，本工具不保证绝对可用。
3. 在提交 Issue、Pull Request 或将调试信息发给其他人时，请注意隐藏个人信息。

# 免责声明
本项目仅供个人学习与交流使用，请遵守学校的校园网使用规定及相关法律法规。使用本工具产生的一切后果由使用者自行承担，开发者不对任何因使用本项目造成的账号封禁、网络问题或其他损失负责。