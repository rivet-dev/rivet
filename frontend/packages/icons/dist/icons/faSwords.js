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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSwords.js
var require_faSwords = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSwords.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "swords";
    var width = 512;
    var height = 512;
    var aliases = [9876];
    var unicode = "f71d";
    var svgPathData = "M96 329.4l50.7-50.7 86.6 86.6L182.6 416l20.7 20.7c6.2 6.2 6.2 16.4 0 22.6l-16 16c-4.7 4.7-11.8 6-17.8 3.3l-62-27.5L51.3 507.3c-6.2 6.2-16.4 6.2-22.6 0l-24-24c-6.2-6.2-6.2-16.4 0-22.6l56.2-56.2-27.5-62c-2.7-6.1-1.4-13.1 3.3-17.8l16-16c6.2-6.2 16.4-6.2 22.6 0L96 329.4zM484.5 114.2L365.3 233.4l-86.6-86.6L397.8 27.5c6.7-6.7 15.2-11.3 24.5-13.1L492.9 .3c5.2-1 10.7 .6 14.5 4.4s5.4 9.2 4.4 14.5L497.6 89.6c-1.9 9.3-6.4 17.8-13.1 24.5zM4.7 4.7C8.5 .9 13.9-.7 19.1 .3L89.6 14.4c9.3 1.9 17.8 6.4 24.5 13.1L393.4 306.7l-86.6 86.6L27.5 114.2c-6.7-6.7-11.3-15.2-13.1-24.5L.3 19.1C-.7 13.9 .9 8.5 4.7 4.7zm454.6 304l16 16c4.7 4.7 6 11.8 3.3 17.8l-27.5 62 56.2 56.2c6.2 6.2 6.2 16.4 0 22.6l-24 24c-6.2 6.2-16.4 6.2-22.6 0l-56.2-56.2-62 27.5c-6.1 2.7-13.1 1.4-17.8-3.3l-16-16c-6.2-6.2-6.2-16.4 0-22.6l128-128c6.2-6.2 16.4-6.2 22.6 0z";
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
    exports.faSwords = exports.definition;
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

// src/_entries/faSwords.js
var import_faSwords = __toESM(require_faSwords());
var export_faSwords = import_faSwords.definition;
export {
  export_faSwords as faSwords
};
