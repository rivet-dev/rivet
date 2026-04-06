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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGlassEmpty.js
var require_faGlassEmpty = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGlassEmpty.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "glass-empty";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e191";
    var svgPathData = "M8.6 10.2C14.6 3.7 23.1 0 32 0L352 0c8.9 0 17.4 3.7 23.4 10.2s9.1 15.2 8.5 24.1L355.1 437.7c-3 41.9-37.8 74.3-79.8 74.3l-166.6 0c-42 0-76.8-32.4-79.8-74.3L.1 34.3c-.6-8.9 2.4-17.6 8.5-24.1zM66.4 64L92.7 433.1c.6 8.4 7.6 14.9 16 14.9l166.6 0c8.4 0 15.4-6.5 16-14.9L317.6 64 66.4 64z";
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
    exports.faGlassEmpty = exports.definition;
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

// src/_entries/faGlassEmpty.js
var import_faGlassEmpty = __toESM(require_faGlassEmpty());
var export_faGlassEmpty = import_faGlassEmpty.definition;
export {
  export_faGlassEmpty as faGlassEmpty
};
