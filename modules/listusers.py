from django.conf import settings

import renderer
from renderer.templates import apply_template
from renderer.utils import get_boolean_param

from ._csrf_protection import csrf_safe_method


def allow_api():
    return True


def has_content():
    return True


@csrf_safe_method
def api_get(context, params):
    if not context.user.is_authenticated:
        tpl_vars = {
            'avatar': settings.DEFAULT_AVATAR,
            'is_authenticated': False
        }
    else:
        tpl_vars = {
            'number': str(context.user.id),
            'title': context.user.username,
            'name': context.user.username,
            'avatar': context.user.get_avatar(settings.DEFAULT_AVATAR),
            'is_authenticated': True
        }
        
    return tpl_vars


def render(context, params, content=None):
    # all params are ignored. always current user

    if not context.user.is_authenticated and not get_boolean_param(params, 'always'):
        return ''
    
    tpl_vars = api_get(context, params)
    template = apply_template((content or '').strip(), tpl_vars)

    return renderer.single_pass_render(template, context)

