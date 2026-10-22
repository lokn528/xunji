/**
 * 不在 Tauri 运行时里时显示的说明页。
 *
 * 循迹是桌面应用：数据在本地 SQLite、文件操作走 Rust 侧、API Key 在系统凭据库。
 * 直接在浏览器里打开构建产物时，`invoke` 全部不可用，页面会一片空白。
 * 与其让人对着白屏猜，不如说清楚这是什么、该怎么打开。
 */
export default function NotInTauri() {
  return (
    <div className="not-in-tauri">
      <div className="not-in-tauri-card">
        <h1>循迹</h1>
        <p className="tagline">所有丢过的东西，有迹可循</p>
        <div className="notice">
          <strong>这是桌面应用，不是网页。</strong>
          <p>
            循迹把卡片存在本机 SQLite，文件操作与 AI
            密钥都放在 Rust 侧处理，浏览器里跑不起来。直接打开网页只会得到一个空壳。
          </p>
        </div>
        <h2>怎么打开</h2>
        <ol>
          <li>
            装好的版本：直接运行{" "}
            <code>src-tauri\target\release\xunji.exe</code>
          </li>
          <li>
            开发模式：在项目目录执行 <code>npm run tauri dev</code>
          </li>
          <li>
            安装包：<code>src-tauri\target\release\bundle\</code> 下的 MSI 或
            NSIS 安装程序
          </li>
        </ol>
        <p className="muted" style={{ fontSize: 12, marginTop: 18 }}>
          数据位置：%APPDATA%\app.xunji.desktop\xunji.db
        </p>
      </div>
    </div>
  );
}
