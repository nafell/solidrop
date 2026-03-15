import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from '@tanstack/react-router'
import { AuthProvider, useAuthContext } from './context/AuthContext'
import { DebugProvider } from './context/DebugContext'
import { router } from './router'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 1, staleTime: 30_000 },
  },
})

function InnerApp() {
  const auth = useAuthContext()
  return <RouterProvider router={router} context={{ auth }} />
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <DebugProvider>
          <InnerApp />
        </DebugProvider>
      </AuthProvider>
    </QueryClientProvider>
  )
}
