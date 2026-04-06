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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faIndustryWindows.js
var require_faIndustryWindows = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faIndustryWindows.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "industry-windows";
    var width = 576;
    var height = 512;
    var aliases = [127981, "industry-alt"];
    var unicode = "f3b3";
    var svgPathData = "M32 64c0-17.7 14.3-32 32-32l64 0c17.7 0 32 14.3 32 32l0 151.4L316.6 131c16-8.6 35.4 3 35.4 21.1l0 63.2L508.6 131c16-8.6 35.4 3 35.4 21.1L544 304l0 128c0 26.5-21.5 48-48 48L80 480c-26.5 0-48-21.5-48-48l0-80 0-48L32 64zM264 312c-8.8 0-16 7.2-16 16l0 48c0 8.8 7.2 16 16 16l48 0c8.8 0 16-7.2 16-16l0-48c0-8.8-7.2-16-16-16l-48 0zM120 328l0 48c0 8.8 7.2 16 16 16l48 0c8.8 0 16-7.2 16-16l0-48c0-8.8-7.2-16-16-16l-48 0c-8.8 0-16 7.2-16 16zm272-16c-8.8 0-16 7.2-16 16l0 48c0 8.8 7.2 16 16 16l48 0c8.8 0 16-7.2 16-16l0-48c0-8.8-7.2-16-16-16l-48 0z";
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
    exports.faIndustryWindows = exports.definition;
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

// src/_entries/faIndustryWindows.js
var import_faIndustryWindows = __toESM(require_faIndustryWindows());
var export_faIndustryWindows = import_faIndustryWindows.definition;
export {
  export_faIndustryWindows as faIndustryWindows
};
