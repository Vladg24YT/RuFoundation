from datetime import datetime, timezone

from modules import ModuleError
from renderer import RenderContext, single_pass_render

from web.events import EventBase
from web.controllers import articles, permissions
from web.models.forum import ForumThread, ForumPost, ForumPostVersion

from ._csrf_protection import csrf_safe_method


class OnForumNewPost(EventBase):
    post: ForumPost
    source: str


def has_content():
    return False


def allow_api():
    return True


@csrf_safe_method
def api_preview(context, params):
    if 'source' not in params:
        raise ModuleError('Исходный код не указан')

    return {'result': single_pass_render(params['source'], RenderContext(None, None, {}, context.user), 'message')}


def api_submit(context, params):
    title = (params.get('name') or '').strip()
    source = (params.get('source') or '').strip()
    reply_to_id = (params.get('replyto') or -1)

    if not source:
        raise ModuleError('Не указан текст сообщения')

    t = params.get('threadid')
    try:
        t = int(t)
        thread = ForumThread.objects.filter(id=t)
        thread = thread[0] if thread else None
    except:
        thread = None

    if thread is None:
        context.status = 404
        raise ModuleError('Тема не найдена или не указана')

    try:
        reply_to_id = int(reply_to_id)
        reply_to = ForumPost.objects.filter(id=reply_to_id)
        reply_to = reply_to[0] if reply_to else None
        if reply_to.thread != thread:
            raise ModuleError('Невозможно создать ответ на сообщение в другой теме')
    except:
        reply_to = None

    if not permissions.check(context.user, 'create', ForumPost(thread=thread)):
        raise ModuleError('Недостаточно прав для создания сообщения')

    post = ForumPost(thread=thread, author=context.user, name=title, reply_to=reply_to)
    post.save()
    post.updated_at = post.created_at
    post.save()

    first_post_content = ForumPostVersion(post=post, source=source, author=context.user)
    first_post_content.save()

    thread.updated_at = datetime.now(timezone.utc)
    thread.save()

    OnForumNewPost(post=post, source=source).emit()

    return {'url': '/forum/t-%d/%s#post-%d' % (thread.id, articles.normalize_article_name(thread.name), post.id)}
