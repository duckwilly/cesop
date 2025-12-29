from fastapi import FastAPI
from fastapi.staticfiles import StaticFiles

from app.core.config import DOCS_DIR, STATIC_DIR
from app.routers import pipeline_router, presentation_router


def create_app() -> FastAPI:
    app = FastAPI(title="CESOP Pipeline Studio", version="0.1.0")
    app.mount("/static", StaticFiles(directory=str(STATIC_DIR)), name="static")
    app.mount("/docs", StaticFiles(directory=str(DOCS_DIR)), name="docs")
    app.include_router(presentation_router)
    app.include_router(pipeline_router)
    return app


app = create_app()
