# Generated by Django 4.0.3 on 2022-05-30 11:08

from django.db import migrations, models


class Migration(migrations.Migration):

    dependencies = [
        ('web', '0008_article_updated_at'),
    ]

    operations = [
        migrations.AlterField(
            model_name='articlelogentry',
            name='meta',
            field=models.JSONField(default=dict),
        ),
        migrations.AlterField(
            model_name='articleversion',
            name='rendered',
            field=models.TextField(blank=True, editable=False, null=True),
        ),
    ]
