/** 与后端 `common::pagination::PaginatedData` 一致；page 从 1 开始 */
export interface PaginatedData<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}
