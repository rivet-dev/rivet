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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHandBackPointDown.js
var require_faHandBackPointDown = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHandBackPointDown.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-back-point-down";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e19e";
    var svgPathData = "M136 512c-22.1 0-40-17.9-40-40l0-200 0-7 0-73c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 55.7c-2-1.4-3.9-3-5.8-4.5L39 227.2C24.4 215 16 197 16 178L16 140c0-38 16.9-74 46.1-98.3l5.4-4.5C96.3 13.1 132.5 0 169.9 0L288 0c70.7 0 128 57.3 128 128l0 64 0 32c0 26.5-21.5 48-48 48c-12.4 0-23.6-4.7-32.1-12.3C334 284.5 313.3 304 288 304c-12.4 0-23.6-4.7-32.1-12.3C254 316.5 233.3 336 208 336c-12.3 0-23.5-4.6-32-12.2L176 472c0 22.1-17.9 40-40 40z";
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
    exports.faHandBackPointDown = exports.definition;
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

// src/_entries/faHandBackPointDown.js
var import_faHandBackPointDown = __toESM(require_faHandBackPointDown());
var export_faHandBackPointDown = import_faHandBackPointDown.definition;
export {
  export_faHandBackPointDown as faHandBackPointDown
};
