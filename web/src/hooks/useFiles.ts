import { useQuery } from '@tanstack/react-query'
import { fetchFiles } from '../api/files'
import { useAuthContext } from '../context/AuthContext'

export function useFiles(prefix?: string) {
  const { apiToken } = useAuthContext()
  return useQuery({
    queryKey: ['files', prefix ?? ''],
    queryFn: () => fetchFiles(apiToken!, prefix),
    enabled: !!apiToken,
  })
}
