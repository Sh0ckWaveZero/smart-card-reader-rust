import { useCardReader } from './hooks/useCardReader'
import { CardInfo } from './components/CardInfo'
import './App.css'

function App() {
  const { cardData, connected, clearCard } = useCardReader()

  return (
    <div className="app">
      <header className="header">
        <h1>Smart Card Reader</h1>
        <div className={`status ${connected ? 'online' : 'offline'}`}>
          <span className="status-dot" />
          {connected ? 'Connected' : 'Disconnected'}
        </div>
      </header>

      <main>
        {cardData ? (
          <div className="card-result">
            <CardInfo data={cardData} />
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
