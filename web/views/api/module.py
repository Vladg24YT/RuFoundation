import modules
from django.http import HttpRequest, HttpResponse

from web.middleware import CsrfViewMiddleware
from web.controllers import articles

from renderer.parser import RenderContext
from . import APIView, takes_json, APIError


class ModuleView(APIView):
    @takes_json
    def post(self, request: HttpRequest, *args, **kwargs) -> HttpResponse:
        data = self.json_input
        if not data or type(data) != dict:
            raise APIError('Некорректный запрос', 400)
        module = data.get('module', None)
        params = data.get('params', {})
        path_params = data.get('pathParams', {})
        page_id = data.get('pageId', None)
        content = data.get('content', None)
        method = data.get('method', None)
        # downcase keys
        params = {key.lower(): value for (key, value) in params.items()}
        path_params = {key.lower(): value for (key, value) in path_params.items()}
        # load page
        article = articles.get_article(page_id)
        context = RenderContext(article, article, path_params, request.user)
        if page_id and not article:
            raise APIError('Страница не найдена', 404)
        # attempt to call module
        try:
            if method == 'render':
                result = modules.render_module(module, context, params, content=content)
                return self.render_json(200, {'result': result})
            else:
                response, is_csrf_safe = modules.handle_api(module, method, context, params)
                if not is_csrf_safe:
                    reason = CsrfViewMiddleware([]).process_view(request, None, (), {})
                    if reason:
                        return reason
                return self.render_json(200, response)
        except modules.ModuleError as e:
            raise APIError(e.message)
