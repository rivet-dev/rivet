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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMessageBot.js
var require_faMessageBot = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMessageBot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "message-bot";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3b8";
    var svgPathData = "M160 0C124.7 0 96 28.7 96 64l0 112-36.3 0c-5.5-9.6-15.9-16-27.7-16c-17.7 0-32 14.3-32 32s14.3 32 32 32c11.8 0 22.2-6.4 27.7-16L96 208l0 144c0 35.3 28.7 64 64 64l64 0 0 80c0 6.1 3.4 11.6 8.8 14.3s11.9 2.1 16.8-1.5L373.3 416 480 416c35.3 0 64-28.7 64-64l0-144 36.3 0c5.5 9.6 15.9 16 27.7 16c17.7 0 32-14.3 32-32s-14.3-32-32-32c-11.8 0-22.2 6.4-27.7 16L544 176l0-112c0-35.3-28.7-64-64-64L160 0zm0 128c0-17.7 14.3-32 32-32l256 0c17.7 0 32 14.3 32 32l0 128c0 17.7-14.3 32-32 32l-256 0c-17.7 0-32-14.3-32-32l0-128zm64 96a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm192 0a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faMessageBot = exports.definition;
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

// src/_entries/faMessageBot.js
var import_faMessageBot = __toESM(require_faMessageBot());
var export_faMessageBot = import_faMessageBot.definition;
export {
  export_faMessageBot as faMessageBot
};
