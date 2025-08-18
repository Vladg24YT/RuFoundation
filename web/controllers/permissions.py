from django.contrib.auth.models import AnonymousUser

from web.models.users import User
from web.models.articles import Article, Category
from web.models.forum import ForumSection, ForumCategory, ForumThread, ForumPost


def check(user, action, obj):
    match (user, action, obj):
        case (User(is_superuser=True) | User(is_staff=True), _, _):
            return True

        case (AnonymousUser(), perm, _) if perm not in ('view', 'view-comments'):
            return False

        case (_, perm, Article(locked=True)) if perm not in ('view', 'comment', 'view-comments', 'rate'):
            return False

        case (User(is_editor=False), 'view', Article(category=category)):
            return get_or_default_category(category).readers_can_view

        case (User(is_editor=False), 'create', Article(category=category)):
            return get_or_default_category(category).readers_can_create

        case (User(is_editor=False), 'edit', Article(category=category)):
            return get_or_default_category(category).readers_can_edit

        case (User(is_editor=False), 'rate', Article(category=category)):
            return get_or_default_category(category).readers_can_rate

        case (User(is_editor=False), 'comment', Article(category=category)):
            return get_or_default_category(category).readers_can_comment

        case (User(is_editor=False), 'view-comments', Article(category=category)):
            return get_or_default_category(category).readers_can_comment

        case (User(is_editor=False), 'delete', Article(category=category)):
            return get_or_default_category(category).users_can_delete

        case (_, 'view', Article(category=category)):
            return get_or_default_category(category).users_can_view

        case (_, 'create', Article(category=category)):
            return get_or_default_category(category).users_can_create

        case (_, 'edit', Article(category=category)):
            return get_or_default_category(category).users_can_edit

        case (_, 'rate', Article(category=category)):
            return get_or_default_category(category).users_can_rate

        case (_, 'comment', Article(category=category)):
            return get_or_default_category(category).users_can_comment

        case (_, 'view-comments', Article(category=category)):
            return get_or_default_category(category).users_can_comment

        case (_, 'delete', Article(category=category)):
            return get_or_default_category(category).users_can_delete

        case (_, 'view', ForumSection(is_hidden_for_users=True)):
            return False

        case (_, 'view', ForumSection()):
            return True

        case (_, 'view', ForumCategory(section=section)):
            return check(user, 'view', section)

        case (User(is_forum_active=False), permission, ForumThread()) if permission != 'view':
            return False

        # check for visibility of article comments
        case (_, 'view', ForumThread(category=None, article=article)):
            return check(user, 'view', article)

        # check for visibility of forum threads
        case (_, 'view', ForumThread(article=None, category=category)):
            return check(user, 'view', category)

        case (_, 'create', ForumThread(category=category)):
            # can see category = can create threads in it
            return check(user, 'view', category)

        case (user, 'edit', ForumThread(author=author)) if author == user:
            return True

        case (user, 'delete', ForumThread(author=author)) if author == user:
            return True

        case (User(is_forum_active=False), permission, ForumPost()) if permission != 'view':
            return False

        # These should not be checked often -- it's going to be quite heavy due to nesting
        case (_, 'create', ForumPost(thread=ForumThread(is_locked=True))):
            return False

        case (_, 'create', ForumPost(thread=ForumThread(article=article))) if article:
            return check(user, 'comment', article)

        case (_, 'create', ForumPost(thread=thread)):
            # Can see thread and thread is not locked = can post
            return check(user, 'view', thread)

        case (_, 'view', ForumPost(thread=thread)):
            return check(user, 'view', thread)

        case (user, 'edit', ForumPost(author=author)) if author == user:
            return True

        case (_, _, _):
            return False


def get_or_default_category(category):
    cat = Category.objects.filter(name=category)
    if not cat:
        return Category(name=category)
    else:
        return cat[0]
