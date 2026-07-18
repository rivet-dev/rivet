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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faEyeDropperHalf.js
var require_faEyeDropperHalf = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faEyeDropperHalf.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "eye-dropper-half";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e173";
    var svgPathData = "M240.1 130.8L341.6 29.2c39-39 102.2-39 141.1 0s39 102.2 0 141.1L381.2 271.9l9.4 9.4c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0l-160-160c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0l9.4 9.4zm-63.9 71.8l45.3 45.3L149.3 320l85.5 0 29.4-29.4 45.3 45.3L188.7 456.6c-15 15-35.4 23.4-56.6 23.4l-42.4 0L49.8 506.6c-12.7 8.5-29.6 6.8-40.4-4s-12.5-27.7-4-40.4L32 422.3l0-42.4c0-21.2 8.4-41.6 23.4-56.6L176.1 202.6z";
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
    exports.faEyeDropperHalf = exports.definition;
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

// src/_entries/faEyeDropperHalf.js
var import_faEyeDropperHalf = __toESM(require_faEyeDropperHalf());
var export_faEyeDropperHalf = import_faEyeDropperHalf.definition;
export {
  export_faEyeDropperHalf as faEyeDropperHalf
};
