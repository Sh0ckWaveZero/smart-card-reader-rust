import { useEffect, useRef, useState, useCallback } from 'react'
import type { ThaiIDData, CardEvent } from '../types'

const WS_URL = 'ws://localhost:8182'
const RECONNECT_INTERVAL = 3000

export function useCardReader() {
  const [cardData, setCardData] = useState<ThaiIDData | null>(null)
  const [connected, setConnected] = useState(false)
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return

    const ws = new WebSocket(WS_URL)

    ws.onopen = () => {
      setConnected(true)
      console.log('WebSocket connected')
    }

    ws.onmessage = (event) => {
      try {
        const msg: CardEvent = JSON.parse(event.data)
        if (msg.type === 'CARD_INSERTED') {
          setCardData(msg.data)
        }
      } catch (e) {
        console.error('Failed to parse message:', e)
      }
    }

    ws.onclose = () => {
      setConnected(false)
      console.log('WebSocket disconnected, reconnecting...')
      reconnectTimer.current = setTimeout(connect, RECONNECT_INTERVAL)
    }

    ws.onerror = () => {
      ws.close()
    }

    wsRef.current = ws
  }, [])

  useEffect(() => {
    connect()
    return () => {
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current)
      wsRef.current?.close()
    }
  }, [connect])

  const clearCard = useCallback(() => setCardData(null), [])

  return { cardData, connected, clearCard }
}
