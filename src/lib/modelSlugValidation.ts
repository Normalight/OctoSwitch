/** 分组别名、绑定路由名中禁止 `/`，否则与客户端 `分组/绑定名` 语法冲突 */
export function segmentHasSlash(s: string): boolean {
  return s.includes("/");
}
