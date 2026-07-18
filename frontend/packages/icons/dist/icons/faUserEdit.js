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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUserPen.js
var require_faUserPen = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUserPen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-pen";
    var width = 640;
    var height = 512;
    var aliases = ["user-edit"];
    var unicode = "f4ff";
    var svgPathData = "M256.1 248a120 120 0 1 0 0-240 120 120 0 1 0 0 240zm-29.7 56c-98.5 0-178.3 79.8-178.3 178.3 0 16.4 13.3 29.7 29.7 29.7l196.5 0 10.9-54.5c4.3-21.7 15-41.6 30.6-57.2l67.3-67.3c-28-18.3-61.4-28.9-97.4-28.9l-59.4 0zM332.3 466.9l-11.9 59.6c-.2 .9-.3 1.9-.3 2.9 0 8 6.5 14.6 14.6 14.6 1 0 1.9-.1 2.9-.3l59.6-11.9c12.4-2.5 23.8-8.6 32.7-17.5l118.9-118.9-80-80-118.9 118.9c-8.9 8.9-15 20.3-17.5 32.7zm267.8-123c22.1-22.1 22.1-57.9 0-80s-57.9-22.1-80 0l-28.8 28.8 80 80 28.8-28.8z";
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
    exports.faUserPen = exports.definition;
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

// src/_entries/faUserEdit.js
var import_faUserPen = __toESM(require_faUserPen());
var export_faUserEdit = import_faUserPen.definition;
export {
  export_faUserEdit as faUserEdit
};
