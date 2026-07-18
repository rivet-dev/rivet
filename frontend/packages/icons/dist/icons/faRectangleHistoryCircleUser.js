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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRectangleHistoryCircleUser.js
var require_faRectangleHistoryCircleUser = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRectangleHistoryCircleUser.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "rectangle-history-circle-user";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e4a4";
    var svgPathData = "M320 368c0 59.5 29.5 112.1 74.8 144L64 512c-35.3 0-64-28.7-64-64L0 224c0-35.3 28.7-64 64-64l384 0c23.8 0 44.5 12.9 55.5 32.2c-2.5-.1-5-.2-7.5-.2c-97.2 0-176 78.8-176 176zM440 80c13.3 0 24 10.7 24 24s-10.7 24-24 24L72 128c-13.3 0-24-10.7-24-24s10.7-24 24-24l368 0zM392 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L120 48c-13.3 0-24-10.7-24-24s10.7-24 24-24L392 0zM352 368a144 144 0 1 1 288 0 144 144 0 1 1 -288 0zm221.7 80.7c-6.2-19-24-32.7-45.1-32.7l-65.2 0c-21 0-38.9 13.7-45.1 32.7C438.5 468.1 465.8 480 496 480s57.5-11.9 77.7-31.3zM544 336a48 48 0 1 0 -96 0 48 48 0 1 0 96 0z";
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
    exports.faRectangleHistoryCircleUser = exports.definition;
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

// src/_entries/faRectangleHistoryCircleUser.js
var import_faRectangleHistoryCircleUser = __toESM(require_faRectangleHistoryCircleUser());
var export_faRectangleHistoryCircleUser = import_faRectangleHistoryCircleUser.definition;
export {
  export_faRectangleHistoryCircleUser as faRectangleHistoryCircleUser
};
