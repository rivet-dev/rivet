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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCherries.js
var require_faCherries = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCherries.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cherries";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e0ec";
    var svgPathData = "M288 24L302.1 4.5c-6.9-5-15.9-5.9-23.7-2.5s-13.2 10.7-14.2 19.1c0 0 0 0 0 0s0 0 0 0s0 0 0 0l0 .2-.1 .9c-.1 .9-.3 2.2-.6 4c-.6 3.6-1.5 9-2.9 15.8c-2.8 13.7-7.6 33.3-15.3 56.3c-14 42-37.4 94.7-75.8 143.7C152.3 233.5 126.5 224 96 224c0 0-96 0-96 112C0 480 144 512 192 512s192-16 192-176c0-112-96-112-96-112c-20.5 0-38.9 4.3-54 9.7c27.1-42.6 45-85.1 56.7-120.2c5.7-17 9.9-32.3 13-45.1c11.7 10.8 25.8 24.7 40.1 41.2c29.6 34.3 59.1 78.4 73.1 128.5c-12-5.3-27-10.3-44.2-12.7C397.3 247.7 416 282.9 416 336c0 71.9-26.1 120.9-62.1 152.9C390.4 506.3 427.7 512 448 512c48 0 192-16 192-176c0-112-96-112-96-112c-32.3 0-59.2 10.6-76.3 19.6C453.5 176 415.2 118.9 380.2 78.3c-19.9-23-39.2-41.3-53.6-53.9c-7.2-6.3-13.2-11.2-17.5-14.5c-2.1-1.7-3.8-3-5-3.9c-.6-.4-1.1-.8-1.4-1l-.4-.3-.1-.1c0 0 0 0 0 0c0 0 0 0 0 0L288 24z";
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
    exports.faCherries = exports.definition;
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

// src/_entries/faCherries.js
var import_faCherries = __toESM(require_faCherries());
var export_faCherries = import_faCherries.definition;
export {
  export_faCherries as faCherries
};
