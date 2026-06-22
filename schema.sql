-- Add new schema named "public"
CREATE SCHEMA IF NOT EXISTS "public";

-- Set comment to schema: "public"
COMMENT ON SCHEMA "public" IS 'standard public schema';

-- Create "__diesel_schema_migrations" table
CREATE TABLE "public"."__diesel_schema_migrations" (
    "version" character varying(50) NOT NULL,
    "run_on" timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY ("version")
);

-- Create enum type "role"
CREATE TYPE "public"."role" AS ENUM ('admin', 'user');

-- Create "users" table
CREATE TABLE "public"."users" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "name" text NOT NULL,
    "email" text NOT NULL,
    "deleted_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id"),
    CONSTRAINT "users_email_key" UNIQUE ("email")
);

-- Create "sessions" table
CREATE TABLE "public"."sessions" (
    "id" text NOT NULL,
    "expires_at" timestamptz NOT NULL,
    "session" text NOT NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id")
);

-- Create index "sessions_expires_at_idx" to table: "sessions"
CREATE INDEX "sessions_expires_at_idx" ON "public"."sessions" ("expires_at");

-- Create "books" table
CREATE TABLE "public"."books" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "name" text NOT NULL,
    "slug" text NOT NULL,
    "owner_id" uuid NOT NULL,
    "deleted_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id"),
    CONSTRAINT "books_slug_key" UNIQUE ("slug"),
    CONSTRAINT "books_owner_id_fkey" FOREIGN KEY ("owner_id") REFERENCES "public"."users" ("id") ON UPDATE NO ACTION ON DELETE NO ACTION
);

-- Create "ingredients" table
CREATE TABLE "public"."ingredients" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "book_id" uuid NOT NULL,
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "name" text NOT NULL,
    "density_g_per_ml" double precision NULL,
    "grocery_section" text NULL,
    "deleted_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id"),
    CONSTRAINT "ingredients_name_key" UNIQUE ("name"),
    CONSTRAINT "ingredients_book_id_fkey" FOREIGN KEY ("book_id") REFERENCES "public"."books" ("id") ON UPDATE NO ACTION ON DELETE CASCADE
);

-- Create index "ingredients_book_id_updated_at_idx" to table: "ingredients"
CREATE INDEX "ingredients_book_id_updated_at_idx" ON "public"."ingredients" ("book_id", "updated_at");

-- Create "meals" table
CREATE TABLE "public"."meals" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "book_id" uuid NOT NULL,
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "slug" text NOT NULL,
    "name" text NOT NULL,
    "deleted_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id"),
    CONSTRAINT "meals_book_id_slug_key" UNIQUE ("book_id", "slug"),
    CONSTRAINT "meals_book_id_fkey" FOREIGN KEY ("book_id") REFERENCES "public"."books" ("id") ON UPDATE NO ACTION ON DELETE CASCADE
);

-- Create index "meals_book_id_updated_at_idx" to table: "meals"
CREATE INDEX "meals_book_id_updated_at_idx" ON "public"."meals" ("book_id", "updated_at");

-- Create "recipes" table
CREATE TABLE "public"."recipes" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "book_id" uuid NOT NULL,
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "slug" text NOT NULL,
    "name" text NOT NULL,
    "source" text NOT NULL,
    "description" text NOT NULL,
    "notes" text NOT NULL,
    "deleted_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id"),
    CONSTRAINT "recipes_book_id_slug_key" UNIQUE ("book_id", "slug"),
    CONSTRAINT "recipes_book_id_fkey" FOREIGN KEY ("book_id") REFERENCES "public"."books" ("id") ON UPDATE NO ACTION ON DELETE CASCADE
);

-- Create index "recipes_book_id_updated_at_idx" to table: "recipes"
CREATE INDEX "recipes_book_id_updated_at_idx" ON "public"."recipes" ("book_id", "updated_at");

-- Create "meal_recipes" table
CREATE TABLE "public"."meal_recipes" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "book_id" uuid NOT NULL,
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "meal_id" uuid NOT NULL,
    "recipe_id" uuid NOT NULL,
    "multiplier" double precision NOT NULL DEFAULT 1.0,
    "position" integer NOT NULL,
    "deleted_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id"),
    CONSTRAINT "meal_recipes_meal_id_position_key" UNIQUE ("meal_id", "position"),
    CONSTRAINT "meal_recipes_book_id_fkey" FOREIGN KEY ("book_id") REFERENCES "public"."books" ("id") ON UPDATE NO ACTION ON DELETE CASCADE,
    CONSTRAINT "meal_recipes_meal_id_fkey" FOREIGN KEY ("meal_id") REFERENCES "public"."meals" ("id") ON UPDATE NO ACTION ON DELETE CASCADE,
    CONSTRAINT "meal_recipes_recipe_id_fkey" FOREIGN KEY ("recipe_id") REFERENCES "public"."recipes" ("id") ON UPDATE NO ACTION ON DELETE NO ACTION
);

-- Create index "meal_recipes_book_id_updated_at_idx" to table: "meal_recipes"
CREATE INDEX "meal_recipes_book_id_updated_at_idx" ON "public"."meal_recipes" ("book_id", "updated_at");

-- Create "recipe_steps" table
CREATE TABLE "public"."recipe_steps" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "book_id" uuid NOT NULL,
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "recipe_id" uuid NOT NULL,
    "position" integer NOT NULL,
    "text" text NOT NULL,
    "duration_s" integer NULL,
    "deleted_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id"),
    CONSTRAINT "recipe_steps_recipe_id_position_key" UNIQUE ("recipe_id", "position"),
    CONSTRAINT "recipe_steps_book_id_fkey" FOREIGN KEY ("book_id") REFERENCES "public"."books" ("id") ON UPDATE NO ACTION ON DELETE CASCADE,
    CONSTRAINT "recipe_steps_recipe_id_fkey" FOREIGN KEY ("recipe_id") REFERENCES "public"."recipes" ("id") ON UPDATE NO ACTION ON DELETE CASCADE
);

-- Create index "recipe_steps_book_id_updated_at_idx" to table: "recipe_steps"
CREATE INDEX "recipe_steps_book_id_updated_at_idx" ON "public"."recipe_steps" ("book_id", "updated_at");

-- Create "recipe_step_ingredients" table
CREATE TABLE "public"."recipe_step_ingredients" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "book_id" uuid NOT NULL,
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "step_id" uuid NOT NULL,
    "position" integer NOT NULL,
    "quantity" double precision NULL,
    "unit_kind" text NULL,
    "unit" text NULL,
    "ingredient_id" uuid NOT NULL,
    "deleted_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id"),
    CONSTRAINT "recipe_step_ingredients_step_id_position_key" UNIQUE ("step_id", "position"),
    CONSTRAINT "recipe_step_ingredients_book_id_fkey" FOREIGN KEY ("book_id") REFERENCES "public"."books" ("id") ON UPDATE NO ACTION ON DELETE CASCADE,
    CONSTRAINT "recipe_step_ingredients_ingredient_id_fkey" FOREIGN KEY ("ingredient_id") REFERENCES "public"."ingredients" ("id") ON UPDATE NO ACTION ON DELETE NO ACTION,
    CONSTRAINT "recipe_step_ingredients_step_id_fkey" FOREIGN KEY ("step_id") REFERENCES "public"."recipe_steps" ("id") ON UPDATE NO ACTION ON DELETE CASCADE
);

-- Create index "recipe_step_ingredients_book_id_updated_at_idx" to table: "recipe_step_ingredients"
CREATE INDEX "recipe_step_ingredients_book_id_updated_at_idx" ON "public"."recipe_step_ingredients" ("book_id", "updated_at");

-- Create "shopping_lists" table
CREATE TABLE "public"."shopping_lists" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "book_id" uuid NOT NULL,
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "slug" text NOT NULL,
    "name" text NOT NULL,
    "deleted_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id"),
    CONSTRAINT "shopping_lists_book_id_slug_key" UNIQUE ("book_id", "slug"),
    CONSTRAINT "shopping_lists_book_id_fkey" FOREIGN KEY ("book_id") REFERENCES "public"."books" ("id") ON UPDATE NO ACTION ON DELETE CASCADE
);

-- Create index "shopping_lists_book_id_updated_at_idx" to table: "shopping_lists"
CREATE INDEX "shopping_lists_book_id_updated_at_idx" ON "public"."shopping_lists" ("book_id", "updated_at");

-- Create "shopping_list_items" table
CREATE TABLE "public"."shopping_list_items" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "book_id" uuid NOT NULL,
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "shopping_list_id" uuid NOT NULL,
    "position" integer NOT NULL,
    "quantity" double precision NULL,
    "unit_kind" text NULL,
    "unit" text NULL,
    "ingredient_id" uuid NULL,
    "text" text NULL,
    "checked" boolean NOT NULL DEFAULT false,
    "deleted_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id"),
    CONSTRAINT "shopping_list_items_shopping_list_id_position_key" UNIQUE ("shopping_list_id", "position"),
    CONSTRAINT "shopping_list_items_book_id_fkey" FOREIGN KEY ("book_id") REFERENCES "public"."books" ("id") ON UPDATE NO ACTION ON DELETE CASCADE,
    CONSTRAINT "shopping_list_items_ingredient_id_fkey" FOREIGN KEY ("ingredient_id") REFERENCES "public"."ingredients" ("id") ON UPDATE NO ACTION ON DELETE NO ACTION,
    CONSTRAINT "shopping_list_items_shopping_list_id_fkey" FOREIGN KEY ("shopping_list_id") REFERENCES "public"."shopping_lists" ("id") ON UPDATE NO ACTION ON DELETE CASCADE
);

-- Create index "shopping_list_items_book_id_updated_at_idx" to table: "shopping_list_items"
CREATE INDEX "shopping_list_items_book_id_updated_at_idx" ON "public"."shopping_list_items" ("book_id", "updated_at");

-- Create "user_passkey_authentications" table
CREATE TABLE "public"."user_passkey_authentications" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "user_id" uuid NOT NULL,
    "passkey_authentication" jsonb NOT NULL,
    PRIMARY KEY ("id"),
    CONSTRAINT "user_passkey_authentications_user_id_key" UNIQUE ("user_id"),
    CONSTRAINT "user_passkey_authentications_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "public"."users" ("id") ON UPDATE NO ACTION ON DELETE CASCADE
);

-- Create "user_passkey_registrations" table
CREATE TABLE "public"."user_passkey_registrations" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "user_id" uuid NOT NULL,
    "passkey_registration" jsonb NOT NULL,
    PRIMARY KEY ("id"),
    CONSTRAINT "user_passkey_registrations_user_id_key" UNIQUE ("user_id"),
    CONSTRAINT "user_passkey_registrations_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "public"."users" ("id") ON UPDATE NO ACTION ON DELETE CASCADE
);

-- Create "user_passkeys" table
CREATE TABLE "public"."user_passkeys" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "deleted_at" timestamptz NULL,
    "user_id" uuid NOT NULL,
    "credential_id" text NOT NULL,
    "passkey" jsonb NOT NULL,
    PRIMARY KEY ("id"),
    CONSTRAINT "user_passkeys_credential_id_key" UNIQUE ("credential_id"),
    CONSTRAINT "user_passkeys_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "public"."users" ("id") ON UPDATE NO ACTION ON DELETE CASCADE
);

-- Create "user_roles" table
CREATE TABLE "public"."user_roles" (
    "id" uuid NOT NULL DEFAULT uuidv7(),
    "book_id" uuid NOT NULL,
    "updated_at" timestamptz NOT NULL DEFAULT NOW(),
    "user_id" uuid NOT NULL,
    "role" "public"."role" NOT NULL,
    "deleted_at" timestamptz NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY ("id"),
    CONSTRAINT "user_roles_book_id_user_id_key" UNIQUE ("book_id", "user_id"),
    CONSTRAINT "user_roles_book_id_fkey" FOREIGN KEY ("book_id") REFERENCES "public"."books" ("id") ON UPDATE NO ACTION ON DELETE CASCADE,
    CONSTRAINT "user_roles_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "public"."users" ("id") ON UPDATE NO ACTION ON DELETE CASCADE
);

-- Create index "user_roles_book_id_updated_at_idx" to table: "user_roles"
CREATE INDEX "user_roles_book_id_updated_at_idx" ON "public"."user_roles" ("book_id", "updated_at");
