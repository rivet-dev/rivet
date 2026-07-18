var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/free-brands-svg-icons/faCloudflare.js
var require_faCloudflare = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCloudflare.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "cloudflare";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e07d";
    var svgPathData = "M407.9 319.9L177.1 317c-.7 0-1.4-.2-2-.5s-1.2-.8-1.6-1.4c-.4-.6-.7-1.3-.7-2s0-1.4 .2-2.1c.4-1.1 1.1-2.1 2.1-2.8s2.1-1.2 3.3-1.2l232.9-2.9c27.6-1.3 57.5-23.6 68-50.8l13.3-34.5c.4-.9 .5-1.9 .5-2.9 0-.5-.1-1.1-.2-1.6-7.4-32.2-25-61.1-50.3-82.3s-56.7-33.7-89.7-35.5-65.6 7.3-93 25.7-48 45.3-58.8 76.5c-11.3-8.5-24.9-13.3-39-13.7s-28 3.5-39.8 11.4-20.8 19.1-25.9 32.3-5.9 27.6-2.4 41.3c-52.3 1.5-94.2 44.1-94.2 96.5 0 4.7 .3 9.3 1 14 .2 1.1 .7 2.1 1.5 2.8s1.9 1.1 2.9 1.1l426.1 .1c0 0 .1 0 .1 0 1.2 0 2.3-.4 3.3-1.1s1.6-1.7 2-2.9l3.3-11.3c3.9-13.4 2.4-25.8-4.1-34.9-6-8.4-16.1-13.3-28.2-13.9zm105.9-98.8c-2.1 0-4.3 .1-6.4 .2-.8 .1-1.5 .3-2.1 .8s-1 1.1-1.3 1.8l-9.1 31.2c-3.9 13.4-2.4 25.8 4.1 34.9 6 8.4 16.1 13.3 28.2 13.9l49.2 2.9c.7 0 1.4 .2 2 .5s1.1 .8 1.5 1.4c.4 .6 .7 1.3 .8 2s0 1.5-.2 2.1c-.4 1.1-1.1 2.1-2.1 2.8s-2.1 1.2-3.3 1.2l-51.1 2.9c-27.8 1.3-57.7 23.6-68.1 50.8l-3.7 9.6c-.2 .4-.2 .8-.2 1.3s.2 .8 .4 1.2 .6 .7 .9 .9 .8 .3 1.2 .3c0 0 .1 0 .1 0l175.9 0c1 0 2-.3 2.8-.9s1.4-1.5 1.7-2.4c3.1-11.1 4.7-22.5 4.7-34 0-69.3-56.5-125.5-126.1-125.5z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faCloudflare = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faCloudflare.js
var import_faCloudflare = __toESM(require_faCloudflare());
var export_faCloudflare = import_faCloudflare.definition;
export {
  export_faCloudflare as faCloudflare
};
