pub fn render_dashboard_html() -> String {
    r##"<!DOCTYPE html>
<html lang="vi">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>SSO Route Tester & OAuth 2.0 PKCE Client</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg-base: #090d16;
            --bg-surface: #101726;
            --bg-elevated: #162035;
            --bg-glass: rgba(16, 23, 38, 0.75);
            --border-subtle: rgba(255, 255, 255, 0.08);
            --border-focus: rgba(99, 102, 241, 0.5);
            
            --primary: #6366f1;
            --primary-hover: #4f46e5;
            --primary-glow: rgba(99, 102, 241, 0.25);
            
            --accent: #06b6d4;
            --success: #10b981;
            --success-glow: rgba(16, 185, 129, 0.2);
            --warning: #f59e0b;
            --warning-glow: rgba(245, 158, 11, 0.2);
            --danger: #ef4444;
            --danger-glow: rgba(239, 68, 68, 0.2);

            --text-primary: #f8fafc;
            --text-secondary: #94a3b8;
            --text-muted: #64748b;
            --font-sans: 'Plus Jakarta Sans', system-ui, -apple-system, sans-serif;
            --font-mono: 'JetBrains Mono', monospace;
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }

        body {
            font-family: var(--font-sans);
            background-color: var(--bg-base);
            color: var(--text-primary);
            min-height: 100vh;
            line-height: 1.5;
            background-image: 
                radial-gradient(at 0% 0%, rgba(99, 102, 241, 0.12) 0px, transparent 50%),
                radial-gradient(at 100% 100%, rgba(6, 182, 212, 0.08) 0px, transparent 50%);
            background-attachment: fixed;
        }

        /* Header */
        header {
            position: sticky;
            top: 0;
            z-index: 100;
            backdrop-filter: blur(16px);
            background: var(--bg-glass);
            border-bottom: 1px solid var(--border-subtle);
            padding: 0.85rem 2rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
        }

        .brand {
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }

        .brand-logo {
            width: 36px;
            height: 36px;
            border-radius: 10px;
            background: linear-gradient(135deg, var(--primary), var(--accent));
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: 800;
            font-size: 1.1rem;
            box-shadow: 0 0 15px var(--primary-glow);
        }

        .brand-title {
            font-size: 1.1rem;
            font-weight: 700;
            letter-spacing: -0.02em;
        }

        .brand-badge {
            font-size: 0.7rem;
            font-weight: 600;
            padding: 0.15rem 0.5rem;
            border-radius: 20px;
            background: rgba(99, 102, 241, 0.2);
            color: #a5b4fc;
            border: 1px solid rgba(99, 102, 241, 0.3);
            text-transform: uppercase;
        }

        .server-target {
            display: flex;
            align-items: center;
            gap: 0.5rem;
            font-size: 0.82rem;
            color: var(--text-secondary);
            background: var(--bg-surface);
            padding: 0.4rem 0.85rem;
            border-radius: 8px;
            border: 1px solid var(--border-subtle);
        }

        .status-dot {
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background: var(--success);
            box-shadow: 0 0 8px var(--success);
        }

        /* Nav Tabs */
        .tabs-bar {
            display: flex;
            gap: 0.5rem;
            padding: 1rem 2rem;
            border-bottom: 1px solid var(--border-subtle);
            background: rgba(16, 23, 38, 0.4);
        }

        .tab-btn {
            background: transparent;
            border: none;
            color: var(--text-secondary);
            font-family: inherit;
            font-size: 0.9rem;
            font-weight: 600;
            padding: 0.6rem 1.1rem;
            border-radius: 8px;
            cursor: pointer;
            transition: all 0.2s ease;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }

        .tab-btn:hover {
            color: var(--text-primary);
            background: rgba(255, 255, 255, 0.04);
        }

        .tab-btn.active {
            color: var(--text-primary);
            background: var(--bg-elevated);
            border: 1px solid var(--border-subtle);
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
        }

        /* Layout Container */
        main {
            max-width: 1300px;
            margin: 0 auto;
            padding: 2rem;
        }

        .tab-content {
            display: none;
        }

        .tab-content.active {
            display: block;
            animation: fadeIn 0.25s ease-out;
        }

        @keyframes fadeIn {
            from { opacity: 0; transform: translateY(6px); }
            to { opacity: 1; transform: translateY(0); }
        }

        /* Cards & Metrics */
        .metrics-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 1.25rem;
            margin-bottom: 2rem;
        }

        .metric-card {
            background: var(--bg-surface);
            border: 1px solid var(--border-subtle);
            border-radius: 12px;
            padding: 1.25rem;
            position: relative;
            overflow: hidden;
        }

        .metric-card::before {
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 2px;
            background: var(--accent-bar, var(--primary));
        }

        .metric-label {
            font-size: 0.8rem;
            font-weight: 600;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }

        .metric-val {
            font-size: 2rem;
            font-weight: 800;
            margin-top: 0.35rem;
            color: var(--text-primary);
            letter-spacing: -0.03em;
        }

        /* Buttons */
        .btn {
            font-family: inherit;
            font-size: 0.88rem;
            font-weight: 600;
            padding: 0.65rem 1.25rem;
            border-radius: 8px;
            border: none;
            cursor: pointer;
            transition: all 0.2s;
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            text-decoration: none;
        }

        .btn-primary {
            background: linear-gradient(135deg, var(--primary), var(--primary-hover));
            color: white;
            box-shadow: 0 4px 14px var(--primary-glow);
        }

        .btn-primary:hover {
            transform: translateY(-1px);
            box-shadow: 0 6px 20px rgba(99, 102, 241, 0.4);
        }

        .btn-secondary {
            background: var(--bg-elevated);
            color: var(--text-primary);
            border: 1px solid var(--border-subtle);
        }

        .btn-secondary:hover {
            background: rgba(255, 255, 255, 0.08);
        }

        .btn-sm {
            padding: 0.35rem 0.75rem;
            font-size: 0.78rem;
        }

        /* Test Items List */
        .section-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 1.25rem;
        }

        .section-title {
            font-size: 1.2rem;
            font-weight: 700;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }

        .test-list {
            display: flex;
            flex-direction: column;
            gap: 0.85rem;
        }

        .test-card {
            background: var(--bg-surface);
            border: 1px solid var(--border-subtle);
            border-radius: 10px;
            padding: 1rem 1.25rem;
            transition: all 0.2s ease;
        }

        .test-card:hover {
            border-color: rgba(255, 255, 255, 0.15);
            background: rgba(22, 32, 53, 0.5);
        }

        .test-main {
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 1rem;
        }

        .test-left {
            display: flex;
            align-items: center;
            gap: 1rem;
            flex-grow: 1;
        }

        .http-method {
            font-family: var(--font-mono);
            font-size: 0.75rem;
            font-weight: 700;
            padding: 0.25rem 0.55rem;
            border-radius: 6px;
            text-transform: uppercase;
        }

        .method-GET { background: rgba(6, 182, 212, 0.15); color: #22d3ee; border: 1px solid rgba(6, 182, 212, 0.3); }
        .method-POST { background: rgba(99, 102, 241, 0.15); color: #818cf8; border: 1px solid rgba(99, 102, 241, 0.3); }

        .test-info {
            display: flex;
            flex-direction: column;
        }

        .test-name {
            font-weight: 600;
            font-size: 0.95rem;
            color: var(--text-primary);
        }

        .test-path {
            font-family: var(--font-mono);
            font-size: 0.8rem;
            color: var(--text-muted);
        }

        .test-right {
            display: flex;
            align-items: center;
            gap: 1rem;
        }

        .status-badge {
            font-size: 0.75rem;
            font-weight: 700;
            padding: 0.25rem 0.65rem;
            border-radius: 20px;
            letter-spacing: 0.04em;
        }

        .status-PASS { background: var(--success-glow); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.3); }
        .status-WARN { background: var(--warning-glow); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.3); }
        .status-FAIL { background: var(--danger-glow); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.3); }
        .status-IDLE { background: rgba(255, 255, 255, 0.05); color: var(--text-muted); border: 1px solid var(--border-subtle); }

        .duration-tag {
            font-family: var(--font-mono);
            font-size: 0.75rem;
            color: var(--text-secondary);
        }

        .test-details {
            margin-top: 0.75rem;
            padding-top: 0.75rem;
            border-top: 1px solid rgba(255, 255, 255, 0.05);
            font-size: 0.85rem;
            color: var(--text-secondary);
        }

        pre.code-block {
            background: #060910;
            border: 1px solid rgba(255, 255, 255, 0.06);
            border-radius: 8px;
            padding: 0.85rem;
            font-family: var(--font-mono);
            font-size: 0.78rem;
            color: #cbd5e1;
            overflow-x: auto;
            margin-top: 0.5rem;
            max-height: 250px;
        }

        /* OAuth Flow Visualizer */
        .flow-diagram {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
            gap: 1rem;
            margin-bottom: 2rem;
        }

        .flow-step {
            background: var(--bg-surface);
            border: 1px solid var(--border-subtle);
            border-radius: 12px;
            padding: 1.25rem;
            position: relative;
        }

        .step-num {
            display: inline-flex;
            width: 24px;
            height: 24px;
            border-radius: 50%;
            background: var(--primary);
            color: white;
            font-size: 0.75rem;
            font-weight: 700;
            align-items: center;
            justify-content: center;
            margin-bottom: 0.75rem;
        }

        .step-title {
            font-weight: 700;
            font-size: 0.95rem;
            margin-bottom: 0.35rem;
        }

        .step-desc {
            font-size: 0.82rem;
            color: var(--text-secondary);
        }

        /* Forms */
        .form-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
            gap: 1.25rem;
            margin-bottom: 1.5rem;
        }

        .form-group {
            display: flex;
            flex-direction: column;
            gap: 0.4rem;
        }

        .form-label {
            font-size: 0.82rem;
            font-weight: 600;
            color: var(--text-secondary);
        }

        .form-input, .form-select, .form-textarea {
            background: var(--bg-surface);
            border: 1px solid var(--border-subtle);
            border-radius: 8px;
            padding: 0.65rem 0.9rem;
            color: var(--text-primary);
            font-family: inherit;
            font-size: 0.88rem;
            transition: all 0.2s;
        }

        .form-input:focus, .form-select:focus, .form-textarea:focus {
            outline: none;
            border-color: var(--border-focus);
            box-shadow: 0 0 10px var(--primary-glow);
        }

        .form-textarea {
            font-family: var(--font-mono);
            resize: vertical;
        }

        /* Banner */
        .alert-banner {
            padding: 1rem 1.25rem;
            border-radius: 10px;
            margin-bottom: 1.5rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
            background: rgba(16, 185, 129, 0.12);
            border: 1px solid rgba(16, 185, 129, 0.3);
            color: #34d399;
        }

        .spinner {
            display: inline-block;
            width: 14px;
            height: 14px;
            border: 2px solid rgba(255, 255, 255, 0.3);
            border-radius: 50%;
            border-top-color: white;
            animation: spin 0.8s linear infinite;
        }

        @keyframes spin {
            to { transform: rotate(360deg); }
        }
    </style>
</head>
<body>

    <!-- Header -->
    <header>
        <div class="brand">
            <div class="brand-logo">S</div>
            <div>
                <div class="brand-title">SSO Test Client & Route Validator</div>
                <div style="font-size: 0.75rem; color: var(--text-muted);">OAuth 2.0 PKCE • End-to-End Test Suite</div>
            </div>
            <span class="brand-badge">Rust Axum</span>
        </div>

        <div style="display: flex; align-items: center; gap: 1rem;">
            <div class="server-target">
                <span class="status-dot"></span>
                <span>SSO Target: <strong id="ssoTargetDisplay">http://localhost:3000</strong></span>
            </div>
            <button class="btn btn-primary btn-sm" onclick="runAllTests()">
                <span>⚡ Chạy toàn bộ Test</span>
            </button>
        </div>
    </header>

    <!-- Nav Tabs -->
    <div class="tabs-bar">
        <button class="tab-btn active" onclick="switchTab('tests')">
            <span>🧪 Bộ Test Routes</span>
        </button>
        <button class="tab-btn" onclick="switchTab('oauth')">
            <span>🔑 Đăng nhập OAuth 2.0 PKCE</span>
        </button>
        <button class="tab-btn" onclick="switchTab('explorer')">
            <span>🚀 API Explorer (Tự do gọi)</span>
        </button>
        <button class="tab-btn" onclick="switchTab('settings')">
            <span>⚙️ Cấu hình Target</span>
        </button>
    </div>

    <!-- Main Container -->
    <main>
        <!-- Success Alert if OAuth returned -->
        <div id="oauthCallbackBanner" class="alert-banner" style="display: none;">
            <div>
                <strong>🎉 OAuth 2.0 PKCE thành công!</strong> Đã nhận authorization code và hoàn tất đổi JWT Access Token.
            </div>
            <button class="btn btn-sm btn-secondary" onclick="this.parentElement.style.display='none'">Đóng</button>
        </div>

        <!-- ==================== TAB 1: TEST SUITE ==================== -->
        <div id="tab-tests" class="tab-content active">
            <!-- Metrics Summary -->
            <div class="metrics-grid">
                <div class="metric-card" style="--accent-bar: var(--primary);">
                    <div class="metric-label">Tổng số Routes</div>
                    <div class="metric-val" id="metricTotal">14</div>
                </div>
                <div class="metric-card" style="--accent-bar: var(--success);">
                    <div class="metric-label">Thành công (PASS)</div>
                    <div class="metric-val" id="metricPassed" style="color: #34d399;">0</div>
                </div>
                <div class="metric-card" style="--accent-bar: var(--warning);">
                    <div class="metric-label">Cảnh báo / Info</div>
                    <div class="metric-val" id="metricWarned" style="color: #fbbf24;">0</div>
                </div>
                <div class="metric-card" style="--accent-bar: var(--danger);">
                    <div class="metric-label">Lỗi (FAIL)</div>
                    <div class="metric-val" id="metricFailed" style="color: #f87171;">0</div>
                </div>
                <div class="metric-card" style="--accent-bar: var(--accent);">
                    <div class="metric-label">Thời gian phản hồi TB</div>
                    <div class="metric-val" id="metricLatency">-- ms</div>
                </div>
            </div>

            <!-- Test List Header -->
            <div class="section-header">
                <div class="section-title">
                    <span>Danh sách Route Tests</span>
                </div>
                <div style="display: flex; gap: 0.5rem;">
                    <button class="btn btn-primary" id="btnRunAll" onclick="runAllTests()">
                        <span>Chạy toàn bộ 14 Tests</span>
                    </button>
                </div>
            </div>

            <!-- Test Cards Container -->
            <div class="test-list" id="testList">
                <div style="padding: 2rem; text-align: center; color: var(--text-muted);">
                    Nhấn "Chạy toàn bộ 14 Tests" để bắt đầu kiểm tra các endpoint của SSO.
                </div>
            </div>
        </div>

        <!-- ==================== TAB 2: OAUTH2 PKCE FLOW ==================== -->
        <div id="tab-oauth" class="tab-content">
            <div class="section-header">
                <div>
                    <h2 class="section-title">Quy trình OAuth 2.0 Authorization Code Flow + PKCE (RFC 7636)</h2>
                    <p style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 0.25rem;">
                        App client này đóng vai trò là một OAuth Client hoàn chỉnh, thực hiện trao đổi khóa bảo mật S256 và nhận JWT token từ SSO IdP.
                    </p>
                </div>
                <button class="btn btn-primary" onclick="startOAuthFlow()">
                    <span>🚀 Bắt đầu đăng nhập SSO với PKCE</span>
                </button>
            </div>

            <!-- Visual Flow Steps -->
            <div class="flow-diagram">
                <div class="flow-step">
                    <div class="step-num">1</div>
                    <div class="step-title">Sinh PKCE Khóa</div>
                    <div class="step-desc">Tạo <code>code_verifier</code> ngẫu nhiên (64 ký tự) và tính <code>code_challenge = SHA256(verifier)</code>.</div>
                </div>
                <div class="flow-step">
                    <div class="step-num">2</div>
                    <div class="step-title">Chuyển hướng /authorize</div>
                    <div class="step-desc">Mở SSO để đăng nhập và cấp quyền, đính kèm <code>code_challenge</code> và <code>code_challenge_method=S256</code>.</div>
                </div>
                <div class="flow-step">
                    <div class="step-num">3</div>
                    <div class="step-title">Nhận Callback & Code</div>
                    <div class="step-desc">SSO redirect về <code>/callback?code=...</code> kèm auth code sử dụng một lần (lưu Redis TTL 10 phút).</div>
                </div>
                <div class="flow-step">
                    <div class="step-num">4</div>
                    <div class="step-title">Đổi Token tại /token</div>
                    <div class="step-desc">Gửi <code>code</code> cùng <code>code_verifier</code> gốc lên SSO để xác thực và nhận JWT Access Token.</div>
                </div>
            </div>

            <!-- Current Session / Token Viewer -->
            <div class="metric-card" style="margin-bottom: 1.5rem;">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
                    <div style="font-weight: 700; font-size: 1.05rem;">Phiên làm việc & Token hiện tại</div>
                    <div style="display: flex; gap: 0.5rem;">
                        <button class="btn btn-secondary btn-sm" onclick="fetchUserInfo()">👤 Gọi /userinfo</button>
                        <button class="btn btn-secondary btn-sm" onclick="clearTokens()">🗑️ Xóa Token</button>
                    </div>
                </div>

                <div id="tokenDetailsContainer" style="color: var(--text-secondary); font-size: 0.9rem;">
                    <em>Chưa có Access Token. Hãy nhấn nút "Bắt đầu đăng nhập SSO với PKCE" ở trên.</em>
                </div>
            </div>

            <!-- OAuth Activity Logs -->
            <div class="section-title" style="font-size: 1rem; margin-bottom: 0.5rem;">Nhật ký OAuth PKCE</div>
            <pre class="code-block" id="oauthLogs" style="height: 160px;">[Sẵn sàng khởi tạo phiên OAuth 2.0 PKCE]</pre>
        </div>

        <!-- ==================== TAB 3: API EXPLORER ==================== -->
        <div id="tab-explorer" class="tab-content">
            <div class="section-header">
                <div>
                    <h2 class="section-title">API Playground & Route Explorer</h2>
                    <p style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 0.25rem;">
                        Tự do gửi HTTP request đến bất kỳ endpoint nào của SSO mà không bị chặn bởi CORS trình duyệt.
                    </p>
                </div>
            </div>

            <div class="form-grid">
                <div class="form-group" style="grid-column: span 1;">
                    <label class="form-label">HTTP Method</label>
                    <select class="form-select" id="explorerMethod">
                        <option value="GET">GET</option>
                        <option value="POST">POST</option>
                    </select>
                </div>

                <div class="form-group" style="grid-column: span 2;">
                    <label class="form-label">Chọn Route mẫu hoặc nhập Endpoint</label>
                    <select class="form-select" id="explorerPreset" onchange="onPresetSelected(this.value)">
                        <option value="/">GET / (Root Redirect)</option>
                        <option value="/api/register">POST /api/register (Đăng ký tài khoản)</option>
                        <option value="/verify_token?email=user@example.com&token=123456">GET /verify_token</option>
                        <option value="/login">GET /login (Giao diện đăng nhập)</option>
                        <option value="/dashboard">GET /dashboard (Bảo vệ bởi Auth Cookie)</option>
                        <option value="/authorize?response_type=code&client_id=test-client&redirect_uri=http://localhost:4000/callback">GET /authorize (OAuth2 Authorize)</option>
                        <option value="/token">POST /token (Đổi mã lấy Access Token)</option>
                        <option value="/userinfo">GET /userinfo (OIDC UserInfo)</option>
                        <option value="/.well-know/jwks.json">GET /.well-know/jwks.json (JWKS Discovery)</option>
                        <option value="/revoke">GET /revoke (Thu hồi Token)</option>
                        <option value="/api/logout">GET /api/logout (Đăng xuất API)</option>
                    </select>
                </div>
            </div>

            <div class="form-group" style="margin-bottom: 1.25rem;">
                <label class="form-label">Đường dẫn đầy đủ</label>
                <input type="text" class="form-input" id="explorerPath" value="/">
            </div>

            <div class="form-group" style="margin-bottom: 1.25rem;">
                <label class="form-label">Request Body (JSON hoặc Form x-www-form-urlencoded - áp dụng cho POST)</label>
                <textarea class="form-textarea" id="explorerBody" rows="4" placeholder='{"email": "test@example.com", "password": "Password123!"}'></textarea>
            </div>

            <button class="btn btn-primary" onclick="sendExplorerRequest()" id="btnSendExplorer">
                <span>Gửi Request</span>
            </button>

            <div style="margin-top: 1.5rem;">
                <div class="section-title" style="font-size: 1rem; margin-bottom: 0.5rem;">Kết quả phản hồi (Response)</div>
                <div id="explorerResultStatus" style="font-weight: 600; margin-bottom: 0.5rem; font-size: 0.88rem; color: var(--text-muted);">
                    Chưa gửi request
                </div>
                <pre class="code-block" id="explorerResultBody" style="min-height: 180px;">// Kết quả phản hồi sẽ hiển thị ở đây</pre>
            </div>
        </div>

        <!-- ==================== TAB 4: SETTINGS ==================== -->
        <div id="tab-settings" class="tab-content">
            <div class="section-header">
                <div>
                    <h2 class="section-title">Cấu hình kết nối SSO</h2>
                    <p style="font-size: 0.85rem; color: var(--text-secondary); margin-top: 0.25rem;">
                        Thay đổi URL đích hoặc thông tin OAuth Client được dùng trong các bài kiểm thử.
                    </p>
                </div>
                <button class="btn btn-primary" onclick="saveSettings()">Lưu Cấu Hình</button>
            </div>

            <div class="form-grid">
                <div class="form-group">
                    <label class="form-label">SSO Base URL</label>
                    <input type="text" class="form-input" id="cfgSsoUrl" value="http://localhost:3000">
                </div>
                <div class="form-group">
                    <label class="form-label">Client ID</label>
                    <input type="text" class="form-input" id="cfgClientId" value="test-client">
                </div>
                <div class="form-group">
                    <label class="form-label">Client Secret (Tùy chọn cho confidential client)</label>
                    <input type="text" class="form-input" id="cfgClientSecret" placeholder="(Để trống nếu là public client)">
                </div>
                <div class="form-group">
                    <label class="form-label">Client Redirect URI (Callback)</label>
                    <input type="text" class="form-input" id="cfgRedirectUri" value="http://localhost:4000/callback">
                </div>
                <div class="form-group" style="grid-column: span 2;">
                    <label class="form-label">OAuth Scopes</label>
                    <input type="text" class="form-input" id="cfgScope" value="openid email profile">
                </div>
            </div>
        </div>
    </main>

    <script>
        // Check for URL parameters
        window.addEventListener('DOMContentLoaded', () => {
            loadConfig();
            loadTokens();
            const urlParams = new URLSearchParams(window.location.search);
            if (urlParams.get('tab') === 'oauth') {
                switchTab('oauth');
                if (urlParams.get('status') === 'success') {
                    document.getElementById('oauthCallbackBanner').style.display = 'flex';
                }
            } else {
                runAllTests();
            }
        });

        function switchTab(tabName) {
            document.querySelectorAll('.tab-btn').forEach(btn => btn.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(tc => tc.classList.remove('active'));

            const targetBtn = Array.from(document.querySelectorAll('.tab-btn')).find(b => b.getAttribute('onclick')?.includes(tabName));
            if (targetBtn) targetBtn.classList.add('active');

            const content = document.getElementById(`tab-${tabName}`);
            if (content) content.classList.add('active');
        }

        async function loadConfig() {
            try {
                const res = await fetch('/api/config');
                const cfg = await res.json();
                document.getElementById('ssoTargetDisplay').textContent = cfg.sso_url;
                document.getElementById('cfgSsoUrl').value = cfg.sso_url;
                document.getElementById('cfgClientId').value = cfg.client_id;
                document.getElementById('cfgClientSecret').value = cfg.client_secret || '';
                document.getElementById('cfgRedirectUri').value = cfg.redirect_uri;
                document.getElementById('cfgScope').value = cfg.scope;
            } catch (e) {
                console.error("Failed to load config", e);
            }
        }

        async function saveSettings() {
            const payload = {
                sso_url: document.getElementById('cfgSsoUrl').value.trim(),
                client_id: document.getElementById('cfgClientId').value.trim(),
                client_secret: document.getElementById('cfgClientSecret').value.trim() || null,
                redirect_uri: document.getElementById('cfgRedirectUri').value.trim(),
                scope: document.getElementById('cfgScope').value.trim()
            };

            try {
                const res = await fetch('/api/config', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });
                if (res.ok) {
                    alert('Đã cập nhật cấu hình thành công!');
                    loadConfig();
                } else {
                    alert('Lỗi cập nhật cấu hình');
                }
            } catch (e) {
                alert('Lỗi: ' + e);
            }
        }

        async function runAllTests() {
            const btn = document.getElementById('btnRunAll');
            btn.disabled = true;
            btn.innerHTML = `<span class="spinner"></span> Đang chạy kiểm thử...`;

            try {
                const res = await fetch('/api/tests/run', { method: 'POST' });
                const tests = await res.json();
                renderTestResults(tests);
            } catch (e) {
                alert('Không thể kết nối đến test server: ' + e);
            } finally {
                btn.disabled = false;
                btn.innerHTML = `Chạy toàn bộ 14 Tests`;
            }
        }

        async function runSingleTest(testId, btn) {
            btn.disabled = true;
            btn.innerHTML = `<span class="spinner"></span>`;

            try {
                const res = await fetch(`/api/tests/run?id=${testId}`, { method: 'POST' });
                const test = await res.json();
                
                // Replace in DOM
                const oldCard = document.getElementById(`card-${testId}`);
                if (oldCard) {
                    oldCard.outerHTML = renderSingleCardHtml(test);
                }
            } catch (e) {
                alert('Lỗi: ' + e);
            }
        }

        function renderTestResults(tests) {
            let passed = 0, warned = 0, failed = 0, totalDuration = 0;
            const container = document.getElementById('testList');
            container.innerHTML = '';

            tests.forEach(test => {
                if (test.status === 'PASS') passed++;
                else if (test.status === 'WARN') warned++;
                else if (test.status === 'FAIL') failed++;
                totalDuration += test.duration_ms;

                container.insertAdjacentHTML('beforeend', renderSingleCardHtml(test));
            });

            document.getElementById('metricTotal').textContent = tests.length;
            document.getElementById('metricPassed').textContent = passed;
            document.getElementById('metricWarned').textContent = warned;
            document.getElementById('metricFailed').textContent = failed;
            document.getElementById('metricLatency').textContent = `${Math.round(totalDuration / (tests.length || 1))} ms`;
        }

        function renderSingleCardHtml(test) {
            const hasBody = test.response_body && test.response_body.length > 0;
            return `
            <div class="test-card" id="card-${test.id}">
                <div class="test-main">
                    <div class="test-left">
                        <span class="http-method method-${test.method}">${test.method}</span>
                        <div class="test-info">
                            <span class="test-name">${test.name}</span>
                            <span class="test-path">${test.path}</span>
                        </div>
                    </div>
                    <div class="test-right">
                        <span class="duration-tag">${test.duration_ms} ms</span>
                        <span class="status-badge status-${test.status}">${test.status}${test.http_status ? ' ' + test.http_status : ''}</span>
                        <button class="btn btn-secondary btn-sm" onclick="runSingleTest('${test.id}', this)">Re-test</button>
                    </div>
                </div>
                <div class="test-details">
                    <div>${test.details}</div>
                    ${hasBody ? `<pre class="code-block">${escapeHtml(test.response_body)}</pre>` : ''}
                </div>
            </div>`;
        }

        function escapeHtml(str) {
            return str.replace(/&/g, '&amp;')
                      .replace(/</g, '&lt;')
                      .replace(/>/g, '&gt;')
                      .replace(/"/g, '&quot;')
                      .replace(/'/g, '&#039;');
        }

        async function startOAuthFlow() {
            try {
                const res = await fetch('/api/oauth/start');
                const data = await res.json();
                if (data.authorize_url) {
                    window.location.href = data.authorize_url;
                } else {
                    alert('Lỗi khởi tạo authorize url');
                }
            } catch (e) {
                alert('Lỗi: ' + e);
            }
        }

        async function loadTokens() {
            try {
                const res = await fetch('/api/oauth/tokens');
                const data = await res.json();

                if (data.logs) {
                    document.getElementById('oauthLogs').textContent = data.logs.join('\n');
                }

                const container = document.getElementById('tokenDetailsContainer');
                if (data.token) {
                    const t = data.token;
                    const u = data.user;
                    container.innerHTML = `
                        <div style="margin-bottom: 0.5rem;">
                            <strong>Token Type:</strong> ${t.token_type} | 
                            <strong>Expires in:</strong> ${t.expires_in}s | 
                            <strong>Scope:</strong> ${t.scope || 'openid'}
                        </div>
                        ${u ? `<div style="margin-bottom: 0.5rem; color: #34d399;"><strong>UserInfo:</strong> Email: ${u.email} (sub: ${u.sub})</div>` : ''}
                        <pre class="code-block" style="word-break: break-all;"><strong>Access Token (JWT):</strong>\n${t.access_token}</pre>
                    `;
                }
            } catch (e) {
                console.error(e);
            }
        }

        async function fetchUserInfo() {
            try {
                const res = await fetch('/api/oauth/userinfo');
                const data = await res.json();
                loadTokens();
                if (data.email) {
                    alert('Đã lấy thông tin UserInfo thành công: ' + data.email);
                } else {
                    alert('UserInfo phản hồi: ' + JSON.stringify(data));
                }
            } catch (e) {
                alert('Lỗi khi gọi UserInfo: ' + e);
            }
        }

        async function clearTokens() {
            await fetch('/api/oauth/clear', { method: 'POST' });
            document.getElementById('tokenDetailsContainer').innerHTML = `<em>Chưa có Access Token. Hãy nhấn nút "Bắt đầu đăng nhập SSO với PKCE" ở trên.</em>`;
            loadTokens();
        }

        function onPresetSelected(val) {
            document.getElementById('explorerPath').value = val;
            if (val.startsWith('/api/register') || val === '/token') {
                document.getElementById('explorerMethod').value = 'POST';
                if (val.startsWith('/api/register')) {
                    document.getElementById('explorerBody').value = JSON.stringify({
                        email: "user_" + Math.floor(Math.random() * 10000) + "@example.com",
                        password: "Password123!"
                    }, null, 2);
                } else {
                    document.getElementById('explorerBody').value = "grant_type=authorization_code&code=SAMPLE_CODE&client_id=test-client&redirect_uri=http://localhost:4000/callback";
                }
            } else {
                document.getElementById('explorerMethod').value = 'GET';
                document.getElementById('explorerBody').value = '';
            }
        }

        async function sendExplorerRequest() {
            const btn = document.getElementById('btnSendExplorer');
            btn.disabled = true;
            btn.innerHTML = `<span class="spinner"></span> Đang gửi...`;

            const payload = {
                method: document.getElementById('explorerMethod').value,
                path: document.getElementById('explorerPath').value.trim(),
                body: document.getElementById('explorerBody').value.trim() || null
            };

            try {
                const res = await fetch('/api/explorer/send', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });
                const result = await res.json();

                const statusEl = document.getElementById('explorerResultStatus');
                statusEl.innerHTML = `Status: <span style="color: ${result.status < 400 ? '#34d399' : '#f87171'}">${result.status} ${result.status_text || ''}</span> • Thời gian: ${result.duration_ms} ms`;
                
                let formatted = result.body;
                try {
                    const parsed = JSON.parse(result.body);
                    formatted = JSON.stringify(parsed, null, 2);
                } catch (_) {}

                document.getElementById('explorerResultBody').textContent = formatted || '(Empty body)';
            } catch (e) {
                alert('Lỗi: ' + e);
            } finally {
                btn.disabled = false;
                btn.innerHTML = `Gửi Request`;
            }
        }
    </script>
</body>
</html>
"##.to_string()
}
