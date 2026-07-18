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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSnowplow.js
var require_faSnowplow = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSnowplow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "snowplow";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f7d2";
    var svgPathData = "M128 160l0-96c0-35.3 28.7-64 64-64L296.4 0c24.2 0 46.4 13.7 57.2 35.4l87.6 175.1c4.4 8.9 6.8 18.7 6.8 28.6l0 56.9 80 0 0-80c0-48.6 39.4-88 88-88 13.3 0 24 10.7 24 24s-10.7 24-24 24c-22.1 0-40 17.9-40 40l0 208c0 22.1 17.9 40 40 40 13.3 0 24 10.7 24 24s-10.7 24-24 24c-48.6 0-88-39.4-88-88l0-80-63 0c9.5 16.5 15 35.6 15 56 0 61.9-50.1 112-112 112l-224 0c-61.9 0-112-50.1-112-112 0-30.5 12.2-58.2 32-78.4L64 224c0-35.3 28.7-64 64-64zm91.7 34.9l43.6 29.1 113.2 0-80-160-104.4 0 0 108.8 27.7 22.1zM416 400c0-26.5-21.5-48-48-48l-224 0c-26.5 0-48 21.5-48 48s21.5 48 48 48l224 0c26.5 0 48-21.5 48-48zm-280 0a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm216-24a24 24 0 1 1 0 48 24 24 0 1 1 0-48zM232 400a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faSnowplow = exports.definition;
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

// src/_entries/faSnowplow.js
var import_faSnowplow = __toESM(require_faSnowplow());
var export_faSnowplow = import_faSnowplow.definition;
export {
  export_faSnowplow as faSnowplow
};
