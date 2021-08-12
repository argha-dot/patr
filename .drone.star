def main(ctx):
    (steps, services) = get_pipeline_steps(ctx)
    branch = ""
    if len(steps) == 0:
        branch = "skip-ci"
    else:
        branch = ctx.build.branch
    return {
        "kind": "pipeline",
        "type": "docker",
        "name": "Default",
        "steps": steps,
        "services": services,

        "trigger": {
            "event": [ctx.build.event],
            "branch": [branch]
        }
    }


def get_pipeline_steps(ctx):
    if is_pr(ctx, "develop"):
        return ([
            # Build in debug mode
            build_code(
                "Build code offline",
                release=False,
                sqlx_offline=True
            ),
            # Check if formatting is fine
            check_formatting("Check formatting"),
            # Check clippy lints
            check_clippy("Check clippy lints"),

            # Create sample config
            copy_config("Copy sample config"),
            # Run --db-only
            init_database(
                "Initialize database",
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=False,
                sqlx_offline=False
            ),
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    elif is_pr(ctx, "staging"):
        return ([
            # Build in release mode
            build_code(
                "Build code offline",
                release=True,
                sqlx_offline=True
            ),
            # Check if formatting is fine
            check_formatting("Check formatting"),
            # Check clippy lints
            check_clippy("Check clippy lints"),

            # Create sample config
            copy_config("Copy sample config"),
            # Run --db-only
            init_database(
                "Initialize database",
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=True,
                sqlx_offline=False
            ),
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    elif is_pr(ctx, "master"):
        return ([
            # Build in release mode
            build_code(
                "Build code offline",
                release=True,
                sqlx_offline=True
            ),
            # Check if formatting is fine
            check_formatting("Check formatting"),
            # Check clippy lints
            check_clippy("Check clippy lints"),

            # Create sample config
            copy_config("Copy sample config"),
            # Run --db-only
            init_database(
                "Initialize database",
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=True,
                sqlx_offline=False
            ),
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    elif is_push(ctx, "develop"):
        return ([
            # Build in debug mode
            build_code(
                "Build code offline",
                release=False,
                sqlx_offline=True
            ),

            # Create sample config
            copy_config(
                "Copy sample config"
            ),
            # Run --db-only
            init_database(
                "Initialize database",
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=False,
                sqlx_offline=False
            ),
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    elif is_push(ctx, "staging"):
        return ([
            # Build in release mode
            build_code(
                "Build code offline",
                release=True,
                sqlx_offline=True
            ),

            # Create sample config
            copy_config("Copy sample config"),
            # Run --db-only
            init_database(
                "Initialize database",
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=True,
                sqlx_offline=False
            ),

            # TODO Deploy
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    elif is_push(ctx, "master"):
        return ([
            # Build in release mode
            build_code(
                "Build code offline",
                release=True,
                sqlx_offline=True
            ),

            # Create sample config
            copy_config("Copy sample config"),
            # Run --db-only
            init_database(
                "Initialize database",
                env=get_app_running_environment()
            ),

            # Clean build cache of `api`
            clean_api_build("Clean build cache"),
            # Run cargo check again, but this time with SQLX_OFFLINE=false
            check_code(
                "Recheck code with live database",
                release=True,
                sqlx_offline=False
            ),

            # TODO Deploy
        ], [
            redis_service(),
            database_service(get_database_password())
        ])
    else:
        return ([], [])


def is_pr(ctx, to_branch):
    return ctx.build.event == "pull_request" and ctx.build.branch == to_branch


def is_push(ctx, on_branch):
    return ctx.build.event == "push" and ctx.build.branch == on_branch


def build_code(step_name, release, sqlx_offline):
    offline = "false"
    if sqlx_offline == True:
        offline = "true"
    else:
        offline = "false"

    release_flag = ""
    if release == True:
        release_flag = "--release"

    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "cargo build {}".format(release_flag)
        ],
        "environment": {
            "SQLX_OFFLINE": offline,
            "DATABASE_URL": "postgres://postgres:{}@database:5432/api".format(
                get_database_password())
        }
    }


def check_formatting(step_name):
    return {
        "name": step_name,
        "image": "rustlang/rust:nightly",
        "commands": [
            "cargo fmt -- --check"
        ]
    }


def check_clippy(step_name):
    return {
        "name": step_name,
        "image": "rustlang/rust:nightly",
        "commands": [
            "cargo clippy -- -D warnings"
        ]
    }


def copy_config(step_name):
    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "cp config/dev.sample.json config/dev.json",
            "cp config/dev.sample.json config/prod.json"
        ]
    }


def init_database(step_name, env):
    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "cargo run -- --db-only"
        ],
        "environment": env
    }


def clean_api_build(step_name):
    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "cargo clean -p api"
        ]
    }


def check_code(step_name, release, sqlx_offline):
    offline = "false"
    if sqlx_offline == True:
        offline = "true"
    else:
        offline = "false"

    release_flag = ""
    if release == True:
        release_flag = "--release"

    return {
        "name": step_name,
        "image": "rust:1",
        "commands": [
            "cargo check {}".format(release_flag)
        ],
        "environment": {
            "SQLX_OFFLINE": offline,
            "DATABASE_URL": "postgres://postgres:{}@database:5432/api".format(
                get_database_password())
        }
    }


def database_service(pwd):
    return {
        "name": "database",
        "image": "postgres",
        "environment": {
            "POSTGRES_PASSWORD": pwd,
            "POSTGRES_DB": "api"
        }
    }


def redis_service():
    return {
        "name": "cache",
        "image": "redis"
    }


def get_database_password():
    return "dAtAbAsEpAsSwOrD"


def get_app_running_environment():
    return {
        "APP_DATABASE_HOST": "database",
        "APP_DATABASE_PORT": 5432,
        "APP_DATABASE_USER": "postgres",
        "APP_DATABASE_PASSWORD": get_database_password(),
        "APP_DATABASE_DATABASE": "api",

        "APP_REDIS_HOST": "cache",
    }
