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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSensorTriangleExclamation.js
var require_faSensorTriangleExclamation = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSensorTriangleExclamation.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sensor-triangle-exclamation";
    var width = 640;
    var height = 512;
    var aliases = ["sensor-alert"];
    var unicode = "e029";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l161 0c-.6-4-1-8.1-1-12.3c0-13.2 3.4-26.2 9.9-37.7l150.4-265c13-22.9 37.3-37.1 63.7-37.1l0-32c0-35.3-28.7-64-64-64L64 32zm48 88l0 144c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-144c0-13.3 10.7-24 24-24s24 10.7 24 24zm96 0l0 144c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-144c0-13.3 10.7-24 24-24s24 10.7 24 24zm53.8 325.8c-3.8 6.7-5.8 14.2-5.8 21.9c0 24.5 19.8 44.3 44.3 44.3l295.4 0c24.5 0 44.3-19.8 44.3-44.3c0-7.7-2-15.2-5.8-21.9l-150.4-265C476.5 168 462.8 160 448 160s-28.5 8-35.8 20.9l-150.4 265zM464 272l0 88c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-88c0-8.8 7.2-16 16-16s16 7.2 16 16zM448 408a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faSensorTriangleExclamation = exports.definition;
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

// src/_entries/faSensorAlert.js
var import_faSensorTriangleExclamation = __toESM(require_faSensorTriangleExclamation());
var export_faSensorAlert = import_faSensorTriangleExclamation.definition;
export {
  export_faSensorAlert as faSensorAlert
};
