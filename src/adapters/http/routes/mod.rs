mod imports;
mod players;
mod saves;
mod system;

use super::app_state::Shared;
use axum::Router;

pub fn router() -> Router<Shared> {
    Router::new()
        .merge(system::routes())
        .merge(saves::routes())
        .merge(imports::routes())
        .merge(players::routes())
}
