import brotli from "brotli";
import CleanCSS from "clean-css";
import fs from "fs/promises";
import {
    Options as HtmlMinifierOptions,
    minify as htmlMinifier,
} from "html-minifier-terser";
import path from "path";
import printf from "printf";
import { MinifyOptions as TerserOptions, minify as jsMinifier } from "terser";

const ORIGINAL_DIR = "../dist";
const MINIFIED_DIR = "../dist-minified";

const terserOptions: TerserOptions = {
    ecma: 2020,
    toplevel: true,
    compress: {
        passes: 3,
    },
};

const htmlMinifierOptions: HtmlMinifierOptions = {
    collapseBooleanAttributes: true,
    collapseWhitespace: true,
    decodeEntities: true,
    html5: true,
    minifyCSS: true,
    minifyJS: terserOptions,
    processConditionalComments: true,
    removeAttributeQuotes: true,
    removeComments: true,
    removeEmptyAttributes: true,
    removeOptionalTags: true,
    removeRedundantAttributes: true,
    removeScriptTypeAttributes: true,
    removeStyleLinkTypeAttributes: true,
    removeTagWhitespace: true,
    sortAttributes: true,
    sortClassName: true,
    trimCustomFragments: true,
    useShortDoctype: true,
};

async function main() {
    await fs.rm(MINIFIED_DIR, { recursive: true, force: true });
    await fs.mkdir(MINIFIED_DIR);

    const processedFiles = [];

    for (const originFileName of await fs.readdir(ORIGINAL_DIR)) {
        const originFilePath = path.join(ORIGINAL_DIR, originFileName);
        const origin = await fs.readFile(originFilePath);

        let minified: Buffer;

        switch (path.extname(originFilePath)) {
            case ".html": {
                minified = Buffer.from(
                    await htmlMinifier(origin.toString(), htmlMinifierOptions),
                );
                break;
            }

            case ".css": {
                minified = Buffer.from(
                    new CleanCSS().minify(origin.toString()).styles,
                );
                break;
            }

            case ".js": {
                const m = await jsMinifier(origin.toString(), terserOptions);
                if (m.code == null) {
                    throw new Error("terser returned nullish code");
                }
                minified = Buffer.from(m.code);
                break;
            }

            default: {
                minified = origin;
            }
        }

        await fs.writeFile(path.join(MINIFIED_DIR, originFileName), minified);
        processedFiles.push(originFileName);
    }

    await showResult(processedFiles);
}

async function showResult(filenames: Array<string>) {
    const filenameMaxLength = filenames
        .map((x) => x.length)
        .reduce((a, b) => Math.max(a, b));

    console.log(
        printf(
            `%${filenameMaxLength}s: %9s %9s %9s`,
            "filename",
            "origin",
            "minify",
            "brotli",
        ),
    );

    for (const filename of filenames) {
        const kib = (hasLength: { length: number }) =>
            printf("%6.02fKiB", hasLength.length / 1024);
        const origin = await fs.readFile(path.join(ORIGINAL_DIR, filename));
        const minified = await fs.readFile(path.join(MINIFIED_DIR, filename));
        const compressed = brotli.compress(minified);

        console.log(
            printf(
                `%${filenameMaxLength}s: %s %s %s`,
                filename,
                kib(origin),
                kib(minified),
                kib(compressed),
            ),
        );
    }
}

main();
