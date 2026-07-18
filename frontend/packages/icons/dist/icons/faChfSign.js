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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChfSign.js
var require_faChfSign = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChfSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chf-sign";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e602";
    var svgPathData = "M0 113.4C0 68.5 36.5 32 81.4 32c30.8 0 59 17.4 72.8 45l2.3 4.7c7.9 15.8 1.5 35-14.3 42.9s-35 1.5-42.9-14.3L97 105.6c-3-5.9-9-9.6-15.6-9.6C71.8 96 64 103.8 64 113.4l0 285.1c0 9.6 7.8 17.4 17.4 17.4c6.6 0 12.6-3.7 15.6-9.6l2.3-4.7c7.9-15.8 27.1-22.2 42.9-14.3s22.2 27.1 14.3 42.9l-2.3 4.7c-13.8 27.6-42 45-72.8 45C36.5 480 0 443.5 0 398.6L0 113.4zM224 32c17.7 0 32 14.3 32 32l0 160 96 0 0-160c0-17.7 14.3-32 32-32s32 14.3 32 32l0 384c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-160-96 0 0 160c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-384c0-17.7 14.3-32 32-32zM448 64c0-17.7 14.3-32 32-32l128 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-96 0 0 128 64 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-64 0 0 160c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-384z";
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
    exports.faChfSign = exports.definition;
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

// src/_entries/faChfSign.js
var import_faChfSign = __toESM(require_faChfSign());
var export_faChfSign = import_faChfSign.definition;
export {
  export_faChfSign as faChfSign
};
