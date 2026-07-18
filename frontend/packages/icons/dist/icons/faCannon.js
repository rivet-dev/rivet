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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCannon.js
var require_faCannon = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCannon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cannon";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e642";
    var svgPathData = "M49.1 353.8L26.5 365.1C2.8 376.9-6.8 405.8 5.1 429.5s40.7 33.3 64.4 21.5L92 439.7c42.7 41.2 108.3 53.2 164.4 25.1c6.1-3.2 14.8-8.1 25.4-14.5C265.5 427 256 398.6 256 368c0-79.5 64.5-144 144-144c52.4 0 98.3 28 123.5 69.9c32.9-21.9 62.9-41.9 86-57.4c26.7-17.9 35.7-52.7 21.4-81.5L600.2 93.5c-14.4-28.7-47.7-42.4-78-31.8C414.8 99.3 177.1 183.2 127.6 207.2c-56.1 28-85.9 87.7-78.5 146.6zM400 480a112 112 0 1 0 0-224 112 112 0 1 0 0 224zm0-144a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faCannon = exports.definition;
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

// src/_entries/faCannon.js
var import_faCannon = __toESM(require_faCannon());
var export_faCannon = import_faCannon.definition;
export {
  export_faCannon as faCannon
};
