from fastapi import APIRouter, Request
from fastapi.templating import Jinja2Templates

from app.core.config import TEMPLATE_DIR

router = APIRouter()

templates = Jinja2Templates(directory=str(TEMPLATE_DIR))


@router.get("/", include_in_schema=False)
def pipeline_home(request: Request):
    return templates.TemplateResponse(
        "presentation/pipeline.html",
        {"request": request},
    )


@router.get("/pipeline", include_in_schema=False)
def pipeline_page(request: Request):
    return templates.TemplateResponse(
        "presentation/pipeline.html",
        {"request": request},
    )
