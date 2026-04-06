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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSubtitlesSlash.js
var require_faSubtitlesSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSubtitlesSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "subtitles-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e610";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-30-23.5c4.6-8.8 7.2-18.9 7.2-29.6l0-320c0-35.3-28.7-64-64-64L96 32c-6.7 0-13.1 1-19.2 2.9L38.8 5.1zm355 278.2l-1.2-.9c-5.3-4.4-8.6-11-8.6-18.4c0-13.3 10.7-24 24-24l80 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-80 0c-5.3 0-10.2-1.7-14.2-4.6zM461 336l27 0c13.3 0 24 10.7 24 24c0 4.8-1.4 9.3-3.8 13L461 336zm26.4 144L365.5 384 312 384c-13.3 0-24-10.7-24-24c0-11.1 7.5-20.4 17.7-23.2l-62-48.8L152 288c-13.3 0-24-10.7-24-24s10.7-24 24-24l30.8 0L32 121.2 32 416c0 35.3 28.7 64 64 64l391.4 0zM152 336l80 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-80 0c-13.3 0-24-10.7-24-24s10.7-24 24-24z";
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
    exports.faSubtitlesSlash = exports.definition;
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

// src/_entries/faSubtitlesSlash.js
var import_faSubtitlesSlash = __toESM(require_faSubtitlesSlash());
var export_faSubtitlesSlash = import_faSubtitlesSlash.definition;
export {
  export_faSubtitlesSlash as faSubtitlesSlash
};
