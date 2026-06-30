import gleam/option.{type Option, None, Some}
import gleam/string
import gleam/list
import gleam/result

/// CMS Content Types
pub type Page {
  Page(
    title: String,
    slug: String,
    body: String,
    published: Bool,
  )
}

pub type CmsState {
  CmsState(
    pages: List(Page),
  )
}

/// Create a new empty CMS state
pub fn init() -> CmsState {
  CmsState(pages: [])
}

/// Add a page if the slug doesn't already exist
pub fn add_page(
  state: CmsState,
  title: String,
  slug: String,
  body: String,
) -> Result(CmsState, String) {
  case find_page(state, slug) {
    Some(_) -> Error("Page with slug '" <> slug <> "' already exists")
    None -> {
      let page = Page(title:, slug:, body:, published: False)
      Ok(CmsState(..state, pages: list.append(state.pages, [page])))
    }
  }
}

/// Find a page by slug
pub fn find_page(
  state: CmsState,
  slug: String,
) -> Option(Page) {
  state.pages
  |> list.find(fn(page) { page.slug == slug })
}

/// Publish a page — set published = True
pub fn publish_page(
  state: CmsState,
  slug: String,
) -> Result(CmsState, String) {
  case find_page(state, slug) {
    None -> Error("Page not found: " <> slug)
    Some(page) -> {
      let published = Page(..page, published: True)
      let pages = list.map(state.pages, fn(p) {
        case p.slug == slug {
          True -> published
          False -> p
        }
      })
      Ok(CmsState(..state, pages:))
    }
  }
}

/// Get all published pages
pub fn published_pages(state: CmsState) -> List(Page) {
  state.pages
  |> list.filter(fn(page) { page.published })
}

/// Count total pages
pub fn page_count(state: CmsState) -> Int {
  list.length(state.pages)
}
