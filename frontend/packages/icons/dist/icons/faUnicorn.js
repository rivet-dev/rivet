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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUnicorn.js
var require_faUnicorn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUnicorn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "unicorn";
    var width = 640;
    var height = 512;
    var aliases = [129412];
    var unicode = "f727";
    var svgPathData = "M448 160l0 78.1 0 1.9s0 0 0 0c-.5 30-12.8 56.4-32 75.7L416 480c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32l0-133.3c-4.1-.7-8.2-1.6-12.3-2.8l-84-24c.2 2.3 .3 4.6 .3 6.9c0 18.6-7.4 36.5-20.6 49.7l-12.7 12.7c-4.8 4.8-6.4 11.8-4.3 18.2l20.8 62.5c6.9 20.7-8.5 42.1-30.4 42.1l-33.7 0c-13.8 0-26-8.8-30.4-21.9L95.3 437.7c-10.6-31.7-6.5-65.9 10-93.9c7.6-12.9 9.5-29.1 1.2-41.6L78.2 259.9c-7.3-11-11.9-23.4-13.6-36.4C54.6 230.8 48 242.6 48 256l0 56c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-56c0-44.6 33.2-81.5 76.3-87.2c14.9-24.4 41.8-40.8 72.5-40.8l54.4 0 20.8 0 96 0C320 57.3 377.3 0 448 0l16 0 16 0 64 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-5.7 0c5.2 4.6 9.6 10 13.1 16l79.3 0c5.1 0 9.3 4.2 9.3 9.3c0 4-2.6 7.6-6.4 8.8L560 90.7l0 64.8c0 22.1-13.4 41.9-33.9 50.1c-26.1 10.4-55.7-.9-68.3-26L448 160zm64-80a16 16 0 1 0 -32 0 16 16 0 1 0 32 0z";
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
    exports.faUnicorn = exports.definition;
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

// src/_entries/faUnicorn.js
var import_faUnicorn = __toESM(require_faUnicorn());
var export_faUnicorn = import_faUnicorn.definition;
export {
  export_faUnicorn as faUnicorn
};
