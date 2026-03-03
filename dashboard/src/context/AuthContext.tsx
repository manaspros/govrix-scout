import { createContext, useContext, useState, useEffect, type ReactNode } from 'react'

interface AuthContextValue {
  darkMode: boolean
  toggleDark: () => void
}

const AuthContext = createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [darkMode, setDarkMode] = useState<boolean>(() => {
    try { return localStorage.getItem('govrix-dark') === 'true' } catch { return false }
  })

  useEffect(() => {
    const html = document.documentElement
    if (darkMode) {
      html.classList.add('dark')
    } else {
      html.classList.remove('dark')
    }
    try { localStorage.setItem('govrix-dark', String(darkMode)) } catch {}
  }, [darkMode])

  const toggleDark = () => setDarkMode(d => !d)

  return (
    <AuthContext.Provider value={{ darkMode, toggleDark }}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used inside AuthProvider')
  return ctx
}
