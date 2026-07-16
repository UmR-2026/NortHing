# NortHing / 绾虫． 浜у搧鍚嶇О绾﹀畾

> **鐢熸晥鏃ユ湡**: 2026-06-25锛坴0.1.0 鍚庣画澶ф敼鍚嶏級
> **鍙栦唬鏂囨。**: `docs/northing-name.md`锛堝墠娆℃敼鍚嶄骇鐗╋紝鏈鍙堟敼鍚嶏級
> **鏇村墠韬?*: `docs/agent-app-name.md`锛坴0.1.0 闃舵锛?> **鍓嶈韩**: `northing` / `Northing`锛坴0.1.0 鍚庣涓€鐗堟敼鍚嶄骇鐗╋級
> **閫傜敤鑼冨洿**: 鎵€鏈変唬鐮併€佹枃妗ｃ€侀厤缃€丆LI 杈撳嚭銆佹棩蹇楁枃浠跺悕

## 閫夊畾鍚嶇О

- **浜у搧鍚嶏紙涓?鑻憋級**: `NortHing` / `绾虫．`
- **CLI 浜岃繘鍒跺悕**: `northhing-cli`
- **妗岄潰浜岃繘鍒跺悕**: `northhing`锛圫lint 澹筹級
- **Server 浜岃繘鍒跺悕**: `northhing-server`
- **Relay 浜岃繘鍒跺悕**: `northhing-relay-server`
- **Internal CLI**: `northhing-internal`锛堢嫭绔?capability-gated CLI锛?- **浠撳簱璺緞**: `E:\agent-project\northhing`锛圓0.x 閲嶅懡鍚嶏級
- **Cargo workspace 鍚?*: `northhing`
- **Crate 鍚嶅墠缂€**: `northhing-*`锛堝叡 27 涓?crate锛?- **鏃ュ織鏂囦欢鍚嶅墠缂€**: `northhing*.log`
- **鍐呴儴鍛藉悕绌洪棿**锛坰nake_case锛? `northhing`
- **鐢ㄦ埛閰嶇疆鐩綍**: `~/.config/northhing/` 鎴?`%APPDATA%\northhing\`
- **Sandbox 鐩綍**: `.northhing/`
- **鐜鍙橀噺鍓嶇紑**: `NORTHHING_*`
- **CLI 鍚姩妯箙**: `NortHing vX.Y.Z`
- **CLI 鍛戒护鍚?*: `northhing`
- **Tauri bundle id**: `com.northhing.installer`
- **GitHub repo**: `UmR-2026/northhing`

## 鏇挎崲瑙勫垯

| 鏃у悕绉?| 鏂板悕绉?| 璇存槑 |
|--------|--------|------|
| `northhing` | `northhing` | 浜у搧鍚嶃€佺敤鎴峰彲瑙佸瓧绗︿覆銆乲ebab-case 閫氱敤 |
| `NortHing` | `NortHing` | prose 涓殑鑻辨枃浜у搧鍚?|
| `northhing's` | `northhing's` | 鎵€鏈夋牸 |
| `northhing` | `northhing` | snake_case Rust crate import / 鍐呴儴鍛藉悕绌洪棿 |
| `NortHing` | `NortHing` | PascalCase Rust 绫诲瀷鍚?|
| `NORTHHING_*` | `NORTHHING_*` | 鍏ㄩ儴澶у啓鐜鍙橀噺 |
| `northhing-*` | `northhing-*` | 27 涓?crate 鍚嶅墠缂€ |
| `northhing_*` | `northhing_*` | Rust snake_case crate 鍚嶏紙鏋佸皯瑙侊級 |
| `opennorthhing` | `opennorthhing` | model provider id锛堢敤鎴峰喅瀹氶噸鍛藉悕锛?|
| `northhing-Installer/` | `northhing-installer/` | installer 鐩綍鍚?|
| `northhing-Installer/src-tauri` | `northhing-installer/src-tauri` | Cargo workspace exclude 璺緞 |
| `northhing://runtime/` | `northhing://runtime/` | tool runtime URI scheme |
| `northhing:embedded` 绛?| `northhing:embedded` 绛?| WebDriver capability 鍚?|
| `NORTHHING_WEBDRIVER_*` | `NORTHHING_WEBDRIVER_*` | webdriver 鐩稿叧 env var |
| `--northhing-*` | `--northhing-*` | CSS custom properties |
| `com.northhing.installer` | `com.northhing.installer` | Tauri bundle identifier |

## 淇濈暀鏃у悕绉扮殑鍦版柟

浠ヤ笅鎯呭喌**涓嶆浛鎹?*锛屼繚鐣欏師濮嬪悕绉?+ 鍔?LEGACY 娉ㄩ噴锛?
- `docs/superpowers/plans/*.md` 涓殑鍘嗗彶 `northhing` 瀛楁牱锛堢敤鎴峰喅绛栵細淇濈暀浣滃巻鍙插弬鑰冿級
- `LICENSE` 鏂囦欢涓殑绗笁鏂圭増鏉冨０鏄?- 涓婃父 fork 鍏崇郴鐨?git remote URL锛堝凡鍒?commit history锛?
## 鍐崇瓥璁板綍

璇﹁ `docs/reviews/2026-06-25-rename-northhing.md`銆?
## 鏌ヨ鏂规硶

```bash
# 鏌ユ壘鎵€鏈夋畫鐣欑殑 "northhing"锛堝簲鍦?rename 鍚庝粎鍛戒腑 docs/superpowers/plans/锛?git grep -in "northhing" -- ':!docs/superpowers/plans/*'

# 鏌ユ壘鎵€鏈?"northhing"锛堝簲涓?0 鍛戒腑锛?git grep -in "northhing"

# 鏌ユ壘鎵€鏈?"NORTHHING_"锛堝簲涓?0 鍛戒腑锛?git grep -in "NORTHHING_"

# 鏌ユ壘鎵€鏈?"northhing-"锛堥櫎鍘嗗彶 plans 澶栧簲涓?0 鍛戒腑锛?git grep -in "northhing-"

# 楠岃瘉 Cargo workspace 鍚?grep "^\[workspace.metadata\]" -A 2 Cargo.toml | grep "name = "
```

## 娉ㄩ噴瑙勮寖

鍦ㄤ唬鐮佷腑寮曠敤浜у搧鍚嶆椂锛屼娇鐢ㄧ粺涓€娉ㄩ噴鏍煎紡锛?
```rust
// northhing: <鎻忚堪>
// 渚嬪锛?// northhing: CLI entry point for the desktop shell
```

瀵逛簬鍘嗗彶浠ｇ爜涓繚鐣欑殑 `northhing` / `northhing` 寮曠敤锛屾坊鍔犳敞閲婅鏄庯細

```rust
// LEGACY(northhing): 淇濈暀鍘熷鍚嶇О锛岃縼绉绘湡鍏煎鎬?// LEGACY(northhing): v0.1.0 涔嬪墠鍓嶈韩
```
