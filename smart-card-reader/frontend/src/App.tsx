import { useState } from 'react'
import { useCardReader } from './hooks/useCardReader'
import { CardInfo } from './components/CardInfo'
import './App.css'

function App() {
  const { cardData, connected, clearCard } = useCardReader()
  const [hidden, setHidden] = useState(true)

  return (
    <div className="app">
      <header className="header">
        <h1>Smart Card Reader</h1>
        <div className="header-right">
          {cardData && (
            <button
              className={`btn-toggle ${hidden ? 'btn-toggle--hidden' : 'btn-toggle--visible'}`}
              onClick={() => setHidden(h => !h)}
              title={hidden ? 'แสดงข้อมูล' : 'ซ่อนข้อมูล'}
            >
              {hidden ? (
                <>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                    <circle cx="12" cy="12" r="3"/>
                  </svg>
                  แสดงข้อมูล
                </>
              ) : (
                <>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94"/>
                    <path d="M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19"/>
                    <line x1="1" y1="1" x2="23" y2="23"/>
                  </svg>
                  ซ่อนข้อมูล
                </>
              )}
            </button>
          )}
          <div className={`status ${connected ? 'online' : 'offline'}`}>
            <span className="status-dot" />
            {connected ? 'Connected' : 'Disconnected'}
          </div>
        </div>
      </header>

      <main>
        {cardData ? (
          <div className="card-result">
            <div className={hidden ? 'card-blurred' : ''}>
              <CardInfo data={cardData} />
            </div>
            <button className="btn-clear" onClick={clearCard}>
              Clear
            </button>
          </div>
        ) : (
          <div className="waiting">
            <div className="waiting-icon">
              <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="#b0a080" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round">
                <rect x="2" y="5" width="20" height="14" rx="2" />
                <line x1="2" y1="10" x2="22" y2="10" />
                <rect x="6" y="13" width="4" height="3" rx="0.5" />
              </svg>
            </div>
            <p className="waiting-title">Waiting for card...</p>
            <p className="waiting-hint">Insert Thai ID card into the reader</p>
          </div>
        )}
      </main>
    </div>
  )
}

export default App
