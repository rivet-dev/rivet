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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCaravan.js
var require_faCaravan = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCaravan.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "caravan";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8ff";
    var svgPathData = "M32 96c0-35.3 28.7-64 64-64l320 0c70.7 0 128 57.3 128 128l0 192 64 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-296.4 0c.2 2.6 .4 5.3 .4 8 0 48.6-39.4 88-88 88s-88-39.4-88-88c0-2.7 .1-5.4 .4-8L96 416c-35.3 0-64-28.7-64-64L32 96zM352 352l64 0c17.7 0 32-14.3 32-32l0-48-24 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l24 0 0-64c0-17.7-14.3-32-32-32l-64 0c-17.7 0-32 14.3-32 32l0 160c0 17.7 14.3 32 32 32zM160 128c-17.7 0-32 14.3-32 32l0 64c0 17.7 14.3 32 32 32l64 0c17.7 0 32-14.3 32-32l0-64c0-17.7-14.3-32-32-32l-64 0zm64 336a40 40 0 1 0 0-80 40 40 0 1 0 0 80z";
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
    exports.faCaravan = exports.definition;
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

// src/_entries/faCaravan.js
var import_faCaravan = __toESM(require_faCaravan());
var export_faCaravan = import_faCaravan.definition;
export {
  export_faCaravan as faCaravan
};
