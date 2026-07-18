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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSpeakers.js
var require_faSpeakers = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSpeakers.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "speakers";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8e0";
    var svgPathData = "M64 0L280.4 0C265.2 17 256 39.4 256 64l0 193.6c-20.3-20.7-48.7-33.6-80-33.6c-61.9 0-112 50.1-112 112s50.1 112 112 112c31.3 0 59.7-12.9 80-33.6l0 33.6c0 24.6 9.2 47 24.4 64L64 512c-35.3 0-64-28.7-64-64L0 64C0 28.7 28.7 0 64 0zM320 503.4c-19.1-11.1-32-31.7-32-55.4l0-112 0-272c0-23.7 12.9-44.4 32-55.4C329.4 3.1 340.3 0 352 0L576 0c35.3 0 64 28.7 64 64l0 384c0 35.3-28.7 64-64 64l-224 0c-11.7 0-22.6-3.1-32-8.6zM352 336a112 112 0 1 0 224 0 112 112 0 1 0 -224 0zM176 272a64 64 0 1 1 0 128 64 64 0 1 1 0-128zm0-112a48 48 0 1 0 0-96 48 48 0 1 0 0 96zM400 336a64 64 0 1 1 128 0 64 64 0 1 1 -128 0zM512 112a48 48 0 1 0 -96 0 48 48 0 1 0 96 0z";
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
    exports.faSpeakers = exports.definition;
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

// src/_entries/faSpeakers.js
var import_faSpeakers = __toESM(require_faSpeakers());
var export_faSpeakers = import_faSpeakers.definition;
export {
  export_faSpeakers as faSpeakers
};
