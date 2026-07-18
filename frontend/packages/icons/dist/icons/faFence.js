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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFence.js
var require_faFence = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFence.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fence";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e303";
    var svgPathData = "M76.8 38.4C73.8 34.4 69 32 64 32s-9.8 2.4-12.8 6.4L6.4 98.1C2.2 103.7 0 110.4 0 117.3L0 448c0 17.7 14.3 32 32 32l64 0c17.7 0 32-14.3 32-32l0-32 32 0 0-64-32 0 0-128 32 0 0-64-32 0 0-42.7c0-6.9-2.2-13.7-6.4-19.2L76.8 38.4zm192 0c-3-4-7.8-6.4-12.8-6.4s-9.8 2.4-12.8 6.4L198.4 98.1c-4.2 5.5-6.4 12.3-6.4 19.2L192 448c0 17.7 14.3 32 32 32l64 0c17.7 0 32-14.3 32-32l0-32 32 0 0-64-32 0 0-128 32 0 0-64-32 0 0-42.7c0-6.9-2.2-13.7-6.4-19.2L268.8 38.4zM448 32c-5 0-9.8 2.4-12.8 6.4L390.4 98.1c-4.2 5.5-6.4 12.3-6.4 19.2L384 448c0 17.7 14.3 32 32 32l64 0c17.7 0 32-14.3 32-32l0-330.7c0-6.9-2.2-13.7-6.4-19.2L460.8 38.4c-3-4-7.8-6.4-12.8-6.4z";
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
    exports.faFence = exports.definition;
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

// src/_entries/faFence.js
var import_faFence = __toESM(require_faFence());
var export_faFence = import_faFence.definition;
export {
  export_faFence as faFence
};
