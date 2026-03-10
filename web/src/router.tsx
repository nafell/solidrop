import {
  createRouter,
  createRootRouteWithContext,
  createRoute,
  redirect,
  Outlet,
} from '@tanstack/react-router'
import type { AuthState } from './context/AuthContext'
import LoginPage from './pages/LoginPage'
import FilesPage from './pages/FilesPage'
import ViewerPage from './pages/ViewerPage'

interface RouterContext {
  auth: AuthState
}

// Root layout with auth context
function RootLayout() {
  return <Outlet />
}

const rootRoute = createRootRouteWithContext<RouterContext>()({
  component: RootLayout,
})

// / → redirect based on auth state
const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  beforeLoad: ({ context }) => {
    if (context.auth.isAuthenticated) {
      throw redirect({ to: '/files' })
    } else {
      throw redirect({ to: '/login' })
    }
  },
})

const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/login',
  beforeLoad: ({ context }) => {
    if (context.auth.isAuthenticated) {
      throw redirect({ to: '/files' })
    }
  },
  component: LoginPage,
})

const filesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/files',
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: FilesPage,
})

const viewerRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/viewer',
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) throw redirect({ to: '/login' })
  },
  component: ViewerPage,
})

const routeTree = rootRoute.addChildren([indexRoute, loginRoute, filesRoute, viewerRoute])

export const router = createRouter({
  routeTree,
  context: {
    auth: {
      isAuthenticated: false,
      masterKey: null,
      apiToken: null,
      login: async () => {},
      logout: () => {},
    },
  },
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
