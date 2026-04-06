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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHeadSideCoughSlash.js
var require_faHeadSideCoughSlash = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHeadSideCoughSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "head-side-cough-slash";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e062";
    var svgPathData = "M535-24.9c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9L41 537.1c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9l25-25 0-447.2c0-15 10.5-28.3 25.4-29.8 7.4-.7 14.9-1.1 22.6-1.1 112.7 0 206 83.3 221.7 191.7 .4 3 1.7 5.9 3.6 8.3l2.2 2.6 227.5-227.5zM133.8 512L288 357.8 288 416c0 53-43 96-96 96l-58.2 0zM176 224a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm336 40a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm-88 72a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm-64 48a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm128 0a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm24 72a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm-88-24a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faHeadSideCoughSlash = exports.definition;
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

// src/_entries/faHeadSideCoughSlash.js
var import_faHeadSideCoughSlash = __toESM(require_faHeadSideCoughSlash());
var export_faHeadSideCoughSlash = import_faHeadSideCoughSlash.definition;
export {
  export_faHeadSideCoughSlash as faHeadSideCoughSlash
};
