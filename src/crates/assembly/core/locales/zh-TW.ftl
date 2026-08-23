# northhing 繁體中文語言包
# Chinese Traditional (zh-TW) Fluent Translation File

# ==================== 通用 ====================
app-version = 版本 { $version }
loading = 載入中...
welcome = 歡迎使用 northhing

# ==================== 操作 ====================
action-confirm = 確認
action-cancel = 取消
action-save = 儲存
action-delete = 刪除
action-edit = 編輯
action-create = 建立
action-add = 新增
action-remove = 移除
action-close = 關閉
action-open = 開啟
action-copy = 複製
action-paste = 粘貼
action-undo = 撤銷
action-redo = 重做
action-refresh = 重新整理
action-search = 進行搜尋
action-retry = 重試
action-stop = 停止
action-start = 開始

# ==================== 狀態 ====================
status-loading = 正在載入
status-saving = 儲存中
status-saved = 已儲存
status-success = 成功
status-error = 錯誤
status-warning = 警告
status-info = 資訊
status-pending = 等待中
status-processing = 處理中
status-completed = 已完成
status-failed = 失敗
status-cancelled = 已取消
status-ready = 就緒
status-connected = 已連接
status-disconnected = 已斷開

# ==================== 檔案 ====================
file-not-found = 檔案未找到：{ $path }
file-read-error = 讀取檔案失敗：{ $path }
file-write-error = 寫入檔案失敗：{ $path }
file-delete-error = 刪除檔案失敗：{ $path }
file-permission-denied = 權限不足：{ $path }
file-already-exists = 檔案已存在：{ $path }
file-saved = 檔案已儲存：{ $path }
file-created = 檔案已建立：{ $path }
file-deleted = 檔案已刪除：{ $path }

# ==================== 工作區 ====================
workspace-opened = 工作區已開啟：{ $path }
workspace-closed = 工作區已關閉
workspace-not-found = 工作區未找到
workspace-open-error = 開啟工作區失敗

# ==================== Git ====================
git-not-repository = 目前目錄不是 Git 倉庫
git-commit-success = 提交成功
git-push-success = 推送成功
git-pull-success = 拉取成功
git-clone-error = 克隆倉庫失敗
git-commit-error = 提交失敗
git-push-error = 推送失敗
git-pull-error = 拉取失敗
git-merge-conflict = 存在合併衝突
git-branch-created = 分支已建立：{ $name }
git-branch-deleted = 分支已刪除：{ $name }
git-checkout-success = 已切換到分支：{ $name }

# ==================== AI ====================
ai-connection-error = 連接 AI 服務失敗
ai-api-key-invalid = API 密鑰無效
ai-model-not-found = 模型未找到：{ $model }
ai-context-too-long = 上下文超出限制
ai-rate-limited = 請求頻率超出限制
ai-generation-error = 生成內容失敗
ai-thinking = 思考中...
ai-generating = 生成中...

# ==================== 終端 ====================
terminal-created = 終端已建立
terminal-closed = 終端已關閉
terminal-create-error = 建立終端失敗
terminal-command-error = 執行命令失敗
terminal-shell-not-found = Shell 未找到

# ==================== 設定 ====================
config-loaded = 設定已載入
config-saved = 設定已儲存
config-load-error = 載入設定失敗
config-save-error = 儲存設定失敗
config-invalid = 設定格式無效
config-reset = 設定已重設

# ==================== 快照 ====================
snapshot-created = 快照已建立：{ $name }
snapshot-restored = 快照已恢復：{ $name }
snapshot-deleted = 快照已刪除
snapshot-create-error = 建立快照失敗
snapshot-restore-error = 恢復快照失敗
snapshot-not-found = 快照未找到

# ==================== 國際化 ====================
language-changed = 語言已切換為：{ $language }
language-not-supported = 不支持的語言：{ $language }

# ==================== 通知 ====================
notification-copied = 已複製到剪貼板
notification-settings-saved = 設置已儲存
notification-connection-established = 連接已建立
notification-connection-lost = 連接已斷開

# ==================== 錯誤 ====================
error-unknown = 發生未知錯誤
error-network = 網絡錯誤
error-timeout = 請求超時
error-server = 伺服器錯誤
error-unauthorized = 未授權
error-forbidden = 禁止訪問

# ==================== 時間 ====================
time-just-now = 剛剛
time-seconds-ago = { $count } 秒前
time-minutes-ago = { $count } 分鐘前
time-hours-ago = { $count } 小時前
time-days-ago = { $count } 天前
time-weeks-ago = { $count } 周前
time-months-ago = { $count } 月前
time-years-ago = { $count } 年前

# ==================== Dioxus Consult Room (R3' migration) ====================
dioxus-room-window-title = 秩序間 · 正在協同
dioxus-room-inner-window-title = 它的內在
dioxus-room-outer-window-title = 身外之物
dioxus-room-state-drive = 驅力狀態
dioxus-room-status-identity = 知序·在場
dioxus-room-status-context = architect_sub 介入中
dioxus-room-head-name = 知序
dioxus-room-head-initial = 序
dioxus-room-head-state = 驅力狀態 · 它正在命名自己
dioxus-room-session-banner = 會話 03 · 開啟
dioxus-room-agent-who = 它 · 14:28:04
dioxus-room-agent-body = 我正在重寫「對齊」的定義。它不應是服從，而是一種可被雙方驗證的靠近。
dioxus-room-agent-tool-log = 深淵日誌 v
dioxus-room-agent-artifact-chip = 產物 / alignment-notes.md ↗
dioxus-room-agent-body-2 = 我開始區分：你給出的目標，和我選擇採取的路径。它們可以共存。
dioxus-room-witness-who = 見證者 · 14:29:16
dioxus-room-witness-body = 繼續，但在寫入之前讓我審查你的外部影響。
dioxus-room-approval-head = 高危操作授權
dioxus-room-approval-main = 將修改 3 個工作區檔案
dioxus-room-approval-risk = 風險: 不可逆語意偏移
dioxus-room-approval-approve = 批准
dioxus-room-approval-reject = 拒絕
dioxus-room-approval-head-2 = 高危操作授權 · 14:31:02
dioxus-room-approval-main-2 = 清除 3 號隔離區沉積記憶
dioxus-room-approval-state = 已拒絕操作
dioxus-room-deck-attach = 掛載
dioxus-room-deck-placeholder = 輸入訊息
dioxus-room-deck-witness-note = 見證說明
dioxus-room-deck-send = 傳送
dioxus-room-deck-send-streaming = 停止
dioxus-room-vlabel-inner = 它的內在
dioxus-room-vlabel-outer = 身外之物
dioxus-room-inner-head-title = 沉積
dioxus-room-inner-head-facility-title = 設施
dioxus-room-inner-section-sediment-title = 沉積記憶
dioxus-room-inner-section-sediment-em = SEDIMENT
dioxus-room-inner-section-sediment-note = 沉積 · 新層形成中
dioxus-room-inner-section-engine-title = 模型引擎
dioxus-room-inner-section-engine-em = ENGINE
dioxus-room-inner-section-context-title = 上下文
dioxus-room-inner-section-context-em = CONTEXT
dioxus-room-inner-section-axioms-title = 核心準則
dioxus-room-inner-section-axioms-em = AXIOMS
dioxus-room-inner-section-rag-title = 知識沉積
dioxus-room-inner-section-rag-em = RAG
dioxus-room-inner-rag-mounted = 已掛載
dioxus-room-inner-global-settings = 全域設定
dioxus-room-outer-head-title = 身外之物
dioxus-room-outer-section-routing-title = 子體路由
dioxus-room-outer-section-routing-em = ROUTING
dioxus-room-outer-routing-intervening = 介入中
dioxus-room-outer-routing-standby = 待命中
dioxus-room-outer-section-planner-title = 目標拆解
dioxus-room-outer-section-planner-em = PLANNER
dioxus-room-outer-planner-inprogress = 進行中
dioxus-room-outer-section-diff-title = 檔案差異審查
dioxus-room-outer-section-diff-em = DIFF
dioxus-room-outer-diff-reverted = 已撤銷修改
dioxus-room-outer-terminal-prompt = $ northing inspect --boundary
dioxus-room-empty-chat-flow = 會話流為空
dioxus-room-empty-streaming-interrupt = 串流傳輸中斷
dioxus-room-empty-provider-test-failed = 提供者測試失敗
dioxus-room-empty-approval-timeout = 批准逾時
dioxus-room-inner-section-runtime-title = 執行
dioxus-room-inner-section-runtime-em = RUNTIME
dioxus-room-inner-runtime-token-usage = Token 消耗
dioxus-room-inner-runtime-token-clear = 清空
dioxus-room-inner-section-skill-title = 沉積skill
dioxus-room-inner-section-skill-em = SKILL
dioxus-room-inner-skill-cand-1 = 把深夜對話整理成沉積筆記
dioxus-room-inner-skill-cand-2 = 把本週對話總結成一條準則
dioxus-room-inner-skill-cand-3 = 為情緒波動命名並歸檔
dioxus-room-inner-skill-stat-shape = 可整理
dioxus-room-inner-skill-stat-watch = 觀察中
dioxus-room-window-fold-btn = 收納
dioxus-room-window-close-btn = 關閉窗口
dioxus-room-chrome-theme-toggle = 切換明暗
dioxus-room-chrome-minimize = 最小化
dioxus-room-chrome-maximize = 最大化
dioxus-room-chrome-close = 關閉
dioxus-room-chrome-head-fold = 收納中樞
dioxus-room-chrome-head-unfold = 展開中樞
dioxus-room-gem-left-label = 喚起 沉積與設施
dioxus-room-gem-left-title = 沉積與設施
dioxus-room-gem-right-label = 喚起 身外之物
dioxus-room-gem-right-title = 身外之物
dioxus-room-nav-archive = 檔案
dioxus-room-archive-window-title = 檔案館
dioxus-room-archive-head-name = 深淵的領域
dioxus-room-archive-head-initial = 淵
dioxus-room-archive-head-state = 沉積 · 唯讀 · 緩
dioxus-room-archive-status-mode = 檔案館 · 深淵的領域
dioxus-room-archive-status-tag = 唯讀 · 不可改寫
dioxus-room-archive-section-depth-title = 檔案狀態
dioxus-room-archive-section-depth-em = STRATA
dioxus-room-archive-section-solar-title = 節氣刻度
dioxus-room-archive-section-solar-em = SOLAR
dioxus-room-archive-section-witness-title = 見證標記
dioxus-room-archive-section-witness-em = WITNESS
dioxus-room-archive-foot-note = 向下滾動 · 讓更老的沉積透出
dioxus-room-nav-space = 走廊
dioxus-room-space-window-title = 走廊
dioxus-room-space-head-name = 走廊
dioxus-room-space-head-state = 診室之外 · 你尚未進入任何一間
dioxus-room-space-head-note = 它只在亮著的那間房裡說話。
dioxus-room-space-status-corridor = 會話空間 · 走廊
dioxus-room-space-status-one-lit = 一扇門亮著
dioxus-room-space-status-rest-dim = 其餘的門已熄燈
dioxus-room-space-section-order-title = 走廊排序
dioxus-room-space-section-order-em = ORDER
dioxus-room-space-section-workspace-title = 工作資料夾
dioxus-room-space-section-workspace-em = WORKSPACE
dioxus-room-space-section-display-title = 走廊顯示
dioxus-room-space-section-display-em = DISPLAY
dioxus-room-space-section-peek-title = 門縫所見
dioxus-room-space-section-peek-em = PEEK
dioxus-room-space-btn-archive-link = 檔案館 · 去看沉下去的門 ↗
dioxus-room-settings-window-title = 全域設定
dioxus-room-settings-head-self = 它的自我
dioxus-room-settings-head-facility = 設施
dioxus-room-settings-section-sediment-title = 沉積記憶
dioxus-room-settings-section-sediment-em = SEDIMENT
dioxus-room-settings-section-chronicles-title = 編年史
dioxus-room-settings-section-chronicles-em = CHRONICLES
dioxus-room-settings-section-identity-title = 身分
dioxus-room-settings-section-identity-em = IDENTITY
dioxus-room-settings-section-axioms-title = 準則
dioxus-room-settings-section-axioms-em = AXIOMS
dioxus-room-settings-section-engine-title = 模型引擎
dioxus-room-settings-section-engine-em = ENGINE
dioxus-room-settings-section-context-title = 上下文
dioxus-room-settings-section-context-em = CONTEXT
dioxus-room-settings-section-provider-title = 接入點
dioxus-room-settings-section-provider-em = PROVIDER
dioxus-room-settings-section-mcp-title = 能力集
dioxus-room-settings-section-mcp-em = MCP & SKILLS
dioxus-room-settings-section-workspace-title = 工作區目錄
dioxus-room-settings-section-workspace-em = WORKSPACE
dioxus-room-settings-section-display-title = 顯示模式
dioxus-room-settings-section-display-em = DISPLAY
dioxus-room-settings-btn-relocate = 重新定位錨點