<div class="split-view-container">
  
  <!-- ================= LIGHT THEME ================= -->
  <div class="preview-pane light-theme">
    <div class="theme-label">Raycast - Light Mode</div>
    <div class="raycast-app-container">
      <div class="raycast-window">
        
        <!-- Sidebar -->
        <aside class="raycast-sidebar">
          <div class="sidebar-header">
            <div class="mac-controls">
              <div class="mac-btn close"></div>
              <div class="mac-btn min"></div>
              <div class="mac-btn max"></div>
            </div>
          </div>
          <div class="nav-section">
            <div class="nav-title">General</div>
            <div class="nav-item active"><span class="icon">✨</span><span class="label">AI Settings</span></div>
            <div class="nav-item"><span class="icon">⚡️</span><span class="label">Extensions</span></div>
            <div class="nav-item"><span class="icon">⌨️</span><span class="label">Shortcuts</span></div>
          </div>
        </aside>

        <!-- Main Content -->
        <main class="raycast-main">
          <header class="main-header">
            <h1 class="page-title">AI Settings</h1>
            <button class="raycast-pill-btn">Save Changes</button>
          </header>

          <div class="content-body">
            <div class="raycast-card">
              <div class="card-left">
                <h3 class="card-heading">Global Model</h3>
                <p class="card-desc">The default Language Model used for standard queries.</p>
              </div>
              <div class="card-right">
                <button class="raycast-secondary-btn">Claude 3.5 Sonnet <span class="arrow">▼</span></button>
              </div>
            </div>

            <div class="raycast-card">
              <div class="card-left">
                <h3 class="card-heading">Command Palette</h3>
                <p class="card-desc">Summon the assistant from anywhere.</p>
              </div>
              <div class="card-right">
                <div class="shortcut-display">
                  <kbd class="raycast-key">⌥</kbd>
                  <kbd class="raycast-key">Space</kbd>
                </div>
              </div>
            </div>

            <div class="section-divider"></div>
            <h2 class="section-title">Danger Zone</h2>
            <div class="raycast-card danger-card">
              <div class="card-left">
                <h3 class="card-heading">Reset All Configurations</h3>
              </div>
              <div class="card-right">
                <button class="raycast-ghost-btn danger">Reset</button>
              </div>
            </div>
          </div>
        </main>
      </div>
    </div>
  </div>


  <!-- ================= DARK THEME ================= -->
  <div class="preview-pane dark-theme">
    <div class="theme-label">Raycast - Dark Mode</div>
    <div class="raycast-app-container">
      <div class="raycast-window">
        
        <!-- Sidebar -->
        <aside class="raycast-sidebar">
          <div class="sidebar-header">
            <div class="mac-controls">
              <div class="mac-btn close"></div>
              <div class="mac-btn min"></div>
              <div class="mac-btn max"></div>
            </div>
          </div>
          <div class="nav-section">
            <div class="nav-title">General</div>
            <div class="nav-item active"><span class="icon">✨</span><span class="label">AI Settings</span></div>
            <div class="nav-item"><span class="icon">⚡️</span><span class="label">Extensions</span></div>
            <div class="nav-item"><span class="icon">⌨️</span><span class="label">Shortcuts</span></div>
          </div>
        </aside>

        <!-- Main Content -->
        <main class="raycast-main">
          <header class="main-header">
            <h1 class="page-title">AI Settings</h1>
            <button class="raycast-pill-btn">Save Changes</button>
          </header>

          <div class="content-body">
            <div class="raycast-card">
              <div class="card-left">
                <h3 class="card-heading">Global Model</h3>
                <p class="card-desc">The default Language Model used for standard queries.</p>
              </div>
              <div class="card-right">
                <button class="raycast-secondary-btn">Claude 3.5 Sonnet <span class="arrow">▼</span></button>
              </div>
            </div>

            <div class="raycast-card">
              <div class="card-left">
                <h3 class="card-heading">Command Palette</h3>
                <p class="card-desc">Summon the assistant from anywhere.</p>
              </div>
              <div class="card-right">
                <div class="shortcut-display">
                  <kbd class="raycast-key">⌥</kbd>
                  <kbd class="raycast-key">Space</kbd>
                </div>
              </div>
            </div>

            <div class="section-divider"></div>
            <h2 class="section-title">Danger Zone</h2>
            <div class="raycast-card danger-card">
              <div class="card-left">
                <h3 class="card-heading">Reset All Configurations</h3>
              </div>
              <div class="card-right">
                <button class="raycast-ghost-btn danger">Reset</button>
              </div>
            </div>
          </div>
        </main>
      </div>
    </div>
  </div>

</div>


<style>
  /* Split View layout */
  .split-view-container {
    display: flex;
    width: 100%;
    height: 100vh;
    gap: 0;
  }

  .preview-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    /* Default preview background is a gray wallpaper */
  }

  .theme-label {
    text-align: center;
    padding: 8px;
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 1px;
    z-index: 10;
  }

  /* Shared App Container */
  .raycast-app-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    box-sizing: border-box;
    font-family: Inter, -apple-system, system-ui, sans-serif;
    font-feature-settings: 'calt', 'kern', 'liga', 'ss03';
    -webkit-font-smoothing: antialiased;
    letter-spacing: 0.2px;
  }

  /* Shared Window Architecture */
  .raycast-window {
    width: 100%;
    max-width: 600px;
    height: 100%;
    max-height: 500px;
    border-radius: 12px;
    display: flex;
    overflow: hidden;
  }

  .raycast-sidebar { width: 180px; display: flex; flex-direction: column; }
  .sidebar-header { height: 52px; padding: 0 16px; display: flex; align-items: center; }
  .mac-controls { display: flex; gap: 8px; }
  .mac-btn { width: 12px; height: 12px; border-radius: 50%; }
  
  .nav-section { padding: 12px 10px; }
  .nav-title { font-size: 11px; font-weight: 600; text-transform: uppercase; margin-bottom: 8px; padding: 0 8px; }
  .nav-item { display: flex; align-items: center; padding: 6px 8px; margin-bottom: 2px; border-radius: 6px; font-size: 13px; font-weight: 500; cursor: pointer; transition: all 0.2s; }
  .nav-item .icon { font-size: 13px; margin-right: 8px; }
  
  .raycast-main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .main-header { height: 60px; padding: 0 24px; display: flex; align-items: center; justify-content: space-between; }
  .page-title { font-size: 18px; font-weight: 500; margin: 0; }
  
  .content-body { padding: 24px; overflow-y: auto; flex: 1; }
  .raycast-card { margin-bottom: 16px; border-radius: 10px; padding: 14px 16px; display: flex; justify-content: space-between; align-items: center; transition: transform 0.1s; }
  .card-heading { font-size: 14px; font-weight: 600; margin: 0 0 4px 0; }
  .card-desc { font-size: 12px; margin: 0; line-height: 1.4; }
  
  .raycast-pill-btn { border: none; font-size: 12px; font-weight: 600; padding: 6px 14px; border-radius: 86px; cursor: pointer; }
  .raycast-secondary-btn { font-size: 12px; font-weight: 500; padding: 5px 12px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 6px; }
  .raycast-secondary-btn .arrow { font-size: 8px; opacity: 0.5; }
  .raycast-ghost-btn { font-size: 12px; font-weight: 500; padding: 5px 12px; border-radius: 6px; border: 1px solid transparent; background: transparent; cursor: pointer; }
  
  .shortcut-display { display: flex; gap: 4px; }
  .raycast-key { font-size: 11px; font-weight: 500; padding: 3px 6px; border-radius: 4px; display: inline-flex; align-items: center; justify-content: center; }
  .section-divider { height: 1px; margin: 24px 0; }
  .section-title { font-size: 12px; font-weight: 600; text-transform: uppercase; margin-bottom: 12px; }


  /* ================= LIGHT THEME SPECIFICS ================= */
  .light-theme { background: #e5e5e5; }
  .light-theme .theme-label { color: #171717; background: #d4d4d4; border-bottom: 1px solid #a3a3a3; }
  .light-theme .raycast-app-container { background: #e8e8ea; } /* macOS desktop light gray */
  
  .light-theme .raycast-window {
    background: #ffffff;
    box-shadow: 
      0px 0px 0px 1px rgba(0, 0, 0, 0.05),
      0px 10px 40px rgba(0, 0, 0, 0.1),
      0px 4px 12px rgba(0, 0, 0, 0.05);
    color: #1c1c1e;
  }

  .light-theme .raycast-sidebar { background: #f3f4f6; border-right: 1px solid rgba(0, 0, 0, 0.08); }
  .light-theme .mac-btn.close { background: #ff5f56; border: 1px solid #e0443e; }
  .light-theme .mac-btn.min { background: #ffbd2e; border: 1px solid #dea123; }
  .light-theme .mac-btn.max { background: #27c93f; border: 1px solid #1aab29; }
  
  .light-theme .nav-title { color: #8e8e93; }
  .light-theme .nav-item { color: #3a3a3c; }
  .light-theme .nav-item:hover { background: rgba(0, 0, 0, 0.04); }
  .light-theme .nav-item.active { background: #e3e4e6; color: #000; box-shadow: 0px 1px 1px rgba(0,0,0,0.02); }
  
  .light-theme .raycast-main { background: #ffffff; }
  .light-theme .main-header { border-bottom: 1px solid rgba(0, 0, 0, 0.06); }
  .light-theme .page-title { color: #000; }
  
  .light-theme .raycast-pill-btn { background: #1c1c1e; color: #fff; box-shadow: 0 1px 2px rgba(0,0,0,0.2); }
  .light-theme .raycast-secondary-btn { background: #ffffff; color: #1c1c1e; border: 1px solid rgba(0, 0, 0, 0.1); box-shadow: 0 1px 2px rgba(0,0,0,0.04); }
  .light-theme .raycast-ghost-btn { color: #8e8e93; }
  .light-theme .raycast-ghost-btn.danger { color: #ff3b30; }
  .light-theme .raycast-ghost-btn.danger:hover { background: rgba(255, 59, 48, 0.1); }
  
  .light-theme .raycast-card { background: #ffffff; border: 1px solid rgba(0, 0, 0, 0.05); box-shadow: 0 2px 8px rgba(0,0,0,0.04); }
  .light-theme .danger-card { border-color: rgba(255, 59, 48, 0.3); background: rgba(255, 59, 48, 0.02); }
  
  .light-theme .card-heading { color: #000; }
  .light-theme .card-desc { color: #8e8e93; }
  
  .light-theme .raycast-key {
    background: linear-gradient(180deg, #ffffff 0%, #f0f0f0 100%);
    color: #3a3a3c;
    box-shadow: 
      0px 1px 1px rgba(0, 0, 0, 0.15),
      0px 0px 0px 1px rgba(0, 0, 0, 0.1),
      inset 0px -1px 0px rgba(0, 0, 0, 0.05);
  }
  .light-theme .section-divider { background: rgba(0, 0, 0, 0.06); }
  .light-theme .section-title { color: #8e8e93; }


  /* ================= DARK THEME SPECIFICS ================= */
  .dark-theme { background: #262626; }
  .dark-theme .theme-label { color: #a3a3a3; background: #171717; border-bottom: 1px solid #404040; }
  .dark-theme .raycast-app-container { background: #000000; }
  
  .dark-theme .raycast-window {
    background: #07080a;
    box-shadow: 
      0px 0px 0px 1px rgba(255, 255, 255, 0.1) inset,
      0px 0px 0px 1px rgba(0, 0, 0, 0.5),
      0px 20px 40px rgba(0, 0, 0, 0.5);
    color: #f9f9f9;
  }

  .dark-theme .raycast-sidebar { background: #101111; border-right: 1px solid rgba(255, 255, 255, 0.06); }
  .dark-theme .mac-btn.close { background: #FF6363; border: 1px solid transparent; }
  .dark-theme .mac-btn.min { background: #ffbc33; border: 1px solid transparent; }
  .dark-theme .mac-btn.max { background: #5fc992; border: 1px solid transparent; }
  
  .dark-theme .nav-title { color: #6a6b6c; }
  .dark-theme .nav-item { color: #9c9c9d; }
  .dark-theme .nav-item:hover { color: #f9f9f9; background: rgba(255,255,255,0.03); }
  .dark-theme .nav-item.active { background: rgba(255, 255, 255, 0.08); color: #f9f9f9; box-shadow: 0px 1px 0px 0px rgba(255, 255, 255, 0.05) inset; }
  
  .dark-theme .raycast-main { background: #07080a; }
  .dark-theme .main-header { border-bottom: 1px solid rgba(255, 255, 255, 0.06); }
  .dark-theme .page-title { color: #f9f9f9; }
  
  .dark-theme .raycast-pill-btn { background: hsla(0, 0%, 100%, 0.85); color: #18191a; }
  .dark-theme .raycast-secondary-btn { background: transparent; color: #f9f9f9; border: 1px solid rgba(255, 255, 255, 0.1); }
  .dark-theme .raycast-ghost-btn { color: #9c9c9d; }
  .dark-theme .raycast-ghost-btn.danger { color: #FF6363; }
  .dark-theme .raycast-ghost-btn.danger:hover { background: hsla(0, 100%, 69%, 0.1); }
  
  .dark-theme .raycast-card { 
    background: #101111; 
    border: 1px solid transparent;
    box-shadow: 
      rgb(27, 28, 30) 0px 0px 0px 1px,
      rgb(7, 8, 10) 0px 0px 0px 1px inset,
      rgba(0, 0, 0, 0.2) 0px 4px 12px; 
  }
  .dark-theme .danger-card { 
    box-shadow: 
      rgb(27, 28, 30) 0px 0px 0px 1px,
      rgba(255, 99, 99, 0.05) 0px 0px 20px inset,
      rgba(0, 0, 0, 0.2) 0px 4px 12px;
  }
  
  .dark-theme .card-heading { color: #f9f9f9; }
  .dark-theme .card-desc { color: #9c9c9d; }
  
  .dark-theme .raycast-key {
    background: linear-gradient(180deg, #1f1f1f 0%, #121212 100%);
    color: #f9f9f9;
    box-shadow: 
      rgba(0, 0, 0, 0.8) 0px 2px 2px,
      rgba(255, 255, 255, 0.1) 0px 1px 0px 0px inset,
      rgba(0, 0, 0, 0.4) 0px -1px 0px 0px inset,
      rgb(7, 8, 10) 0px 0px 0px 1px;
  }
  .dark-theme .section-divider { background: #252829; }
  .dark-theme .section-title { color: #6a6b6c; }
</style>
