import { NextJsSite } from "./nextjs";

const site = new NextJsSite("crisp", {
    path: "crisp",
});

export const url = site.url;