# northhing 简体中文语言包
# Chinese Simplified (zh-CN) Fluent Translation File

# ==================== 通用 ====================
app-version = 版本 { $version }
loading = 加载中...
welcome = 欢迎使用 northhing

# ==================== 操作 ====================
action-confirm = 确认
action-cancel = 取消
action-save = 保存
action-delete = 删除
action-edit = 编辑
action-create = 创建
action-add = 添加
action-remove = 移除
action-close = 关闭
action-open = 打开
action-copy = 复制
action-paste = 粘贴
action-undo = 撤销
action-redo = 重做
action-refresh = 刷新
action-search = 搜索
action-retry = 重试
action-stop = 停止
action-start = 开始

# ==================== 状态 ====================
status-loading = 加载中
status-saving = 保存中
status-saved = 已保存
status-success = 成功
status-error = 错误
status-warning = 警告
status-info = 信息
status-pending = 等待中
status-processing = 处理中
status-completed = 已完成
status-failed = 失败
status-cancelled = 已取消
status-ready = 就绪
status-connected = 已连接
status-disconnected = 已断开

# ==================== 文件 ====================
file-not-found = 文件未找到：{ $path }
file-read-error = 读取文件失败：{ $path }
file-write-error = 写入文件失败：{ $path }
file-delete-error = 删除文件失败：{ $path }
file-permission-denied = 权限不足：{ $path }
file-already-exists = 文件已存在：{ $path }
file-saved = 文件已保存：{ $path }
file-created = 文件已创建：{ $path }
file-deleted = 文件已删除：{ $path }

# ==================== 工作区 ====================
workspace-opened = 工作区已打开：{ $path }
workspace-closed = 工作区已关闭
workspace-not-found = 工作区未找到
workspace-open-error = 打开工作区失败

# ==================== Git ====================
git-not-repository = 当前目录不是 Git 仓库
git-commit-success = 提交成功
git-push-success = 推送成功
git-pull-success = 拉取成功
git-clone-error = 克隆仓库失败
git-commit-error = 提交失败
git-push-error = 推送失败
git-pull-error = 拉取失败
git-merge-conflict = 存在合并冲突
git-branch-created = 分支已创建：{ $name }
git-branch-deleted = 分支已删除：{ $name }
git-checkout-success = 已切换到分支：{ $name }

# ==================== AI ====================
ai-connection-error = 连接 AI 服务失败
ai-api-key-invalid = API 密钥无效
ai-model-not-found = 模型未找到：{ $model }
ai-context-too-long = 上下文超出限制
ai-rate-limited = 请求频率超出限制
ai-generation-error = 生成内容失败
ai-thinking = 思考中...
ai-generating = 生成中...

# ==================== 终端 ====================
terminal-created = 终端已创建
terminal-closed = 终端已关闭
terminal-create-error = 创建终端失败
terminal-command-error = 执行命令失败
terminal-shell-not-found = Shell 未找到

# ==================== 配置 ====================
config-loaded = 配置已加载
config-saved = 配置已保存
config-load-error = 加载配置失败
config-save-error = 保存配置失败
config-invalid = 配置格式无效
config-reset = 配置已重置

# ==================== 快照 ====================
snapshot-created = 快照已创建：{ $name }
snapshot-restored = 快照已恢复：{ $name }
snapshot-deleted = 快照已删除
snapshot-create-error = 创建快照失败
snapshot-restore-error = 恢复快照失败
snapshot-not-found = 快照未找到

# ==================== 国际化 ====================
language-changed = 语言已切换为：{ $language }
language-not-supported = 不支持的语言：{ $language }

# ==================== 通知 ====================
notification-copied = 已复制到剪贴板
notification-settings-saved = 设置已保存
notification-connection-established = 连接已建立
notification-connection-lost = 连接已断开

# ==================== 错误 ====================
error-unknown = 发生未知错误
error-network = 网络错误
error-timeout = 请求超时
error-server = 服务器错误
error-unauthorized = 未授权
error-forbidden = 禁止访问

# ==================== 时间 ====================
time-just-now = 刚刚
time-seconds-ago = { $count } 秒前
time-minutes-ago = { $count } 分钟前
time-hours-ago = { $count } 小时前
time-days-ago = { $count } 天前
time-weeks-ago = { $count } 周前
time-months-ago = { $count } 月前
time-years-ago = { $count } 年前

# ==================== Dioxus Consult Room (R3' migration) ====================
dioxus-room-window-title = 秩序间 · 正在协同
dioxus-room-inner-window-title = 它的内在
dioxus-room-outer-window-title = 身外之物
dioxus-room-state-drive = 驱力状态
dioxus-room-status-identity = 知序·在场
dioxus-room-status-context = architect_sub 介入中
dioxus-room-head-name = 知序
dioxus-room-head-initial = 序
dioxus-room-head-state = 驱力状态 · 它正在命名自己
dioxus-room-session-banner = 会话 03 · 开启
dioxus-room-agent-who = 它 · 14:28:04
dioxus-room-agent-body = 我正在重写"对齐"的定义。它不应是服从，而是一种可被双方验证的靠近。
dioxus-room-agent-tool-log = 深渊日志 v
dioxus-room-agent-artifact-chip = 产物 / alignment-notes.md ↗
dioxus-room-agent-body-2 = 我开始区分：你给出的目标，和我选择采取的路径。它们可以共存。
dioxus-room-witness-who = 见证者 · 14:29:16
dioxus-room-witness-body = 继续，但在写入之前让我审查你的外部影响。
dioxus-room-approval-head = 高危操作授权
dioxus-room-approval-main = 将修改 3 个工作区文件
dioxus-room-approval-risk = 风险: 不可逆语义偏移
dioxus-room-approval-approve = 批准
dioxus-room-approval-reject = 拒绝
dioxus-room-approval-head-2 = 高危操作授权 · 14:31:02
dioxus-room-approval-main-2 = 清除 3 号隔离区沉积记忆
dioxus-room-approval-state = 已拒绝操作
dioxus-room-deck-attach = 挂载
dioxus-room-deck-placeholder = 输入消息
dioxus-room-deck-witness-note = 见证说明
dioxus-room-deck-send = 发送
dioxus-room-deck-send-streaming = 停止
dioxus-room-vlabel-inner = 它的内在
dioxus-room-vlabel-outer = 身外之物
dioxus-room-inner-head-title = 它的自我
dioxus-room-inner-head-facility-title = 设施
dioxus-room-inner-section-sediment-title = 沉积记忆
dioxus-room-inner-section-sediment-em = SEDIMENT
dioxus-room-inner-section-sediment-note = 沉积 · 新层形成中
dioxus-room-inner-section-engine-title = 模型引擎
dioxus-room-inner-section-engine-em = ENGINE
dioxus-room-inner-section-context-title = 上下文
dioxus-room-inner-section-context-em = CONTEXT
dioxus-room-inner-section-axioms-title = 核心准则
dioxus-room-inner-section-axioms-em = AXIOMS
dioxus-room-inner-section-rag-title = 知识沉积
dioxus-room-inner-section-rag-em = RAG
dioxus-room-inner-rag-mounted = 已挂载
dioxus-room-inner-global-settings = 全局设置
dioxus-room-outer-head-title = 身外之物
dioxus-room-outer-section-routing-title = 子体路由
dioxus-room-outer-section-routing-em = ROUTING
dioxus-room-outer-routing-intervening = 介入中
dioxus-room-outer-routing-standby = 待命中
dioxus-room-outer-section-planner-title = 目标拆解
dioxus-room-outer-section-planner-em = PLANNER
dioxus-room-outer-planner-inprogress = 进行中
dioxus-room-outer-section-diff-title = 文件差异审查
dioxus-room-outer-section-diff-em = DIFF
dioxus-room-outer-diff-reverted = 已撤销修改
dioxus-room-outer-terminal-prompt = $ northing inspect --boundary
dioxus-room-empty-chat-flow = 会话流为空
dioxus-room-empty-streaming-interrupt = 流式传输中断
dioxus-room-empty-provider-test-failed = 提供者测试失败
dioxus-room-empty-approval-timeout = 批准超时
