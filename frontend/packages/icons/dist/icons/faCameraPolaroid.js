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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraPolaroid.js
var require_faCameraPolaroid = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraPolaroid.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "camera-polaroid";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f8aa";
    var svgPathData = "M64 96c0-35.3 28.7-64 64-64l320 0c35.3 0 64 28.7 64 64l0 150.3 53.3 79.9c7 10.5 10.7 22.9 10.7 35.5l0 54.3c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64l0-54.3c0-12.6 3.7-25 10.7-35.5L64 246.3 64 96zM288 256a64 64 0 1 0 0-128 64 64 0 1 0 0 128zM448 128a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zM112 384c-8.8 0-16 7.2-16 16s7.2 16 16 16l352 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-352 0z";
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
    exports.faCameraPolaroid = exports.definition;
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

// src/_entries/faCameraPolaroid.js
var import_faCameraPolaroid = __toESM(require_faCameraPolaroid());
var export_faCameraPolaroid = import_faCameraPolaroid.definition;
export {
  export_faCameraPolaroid as faCameraPolaroid
};
