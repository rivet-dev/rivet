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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTrashCanList.js
var require_faTrashCanList = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTrashCanList.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "trash-can-list";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e2ab";
    var svgPathData = "M147.8 0c-12.1 0-23.2 6.8-28.6 17.7L112 32 32 32C14.3 32 0 46.3 0 64S14.3 96 32 96l352 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-80 0-7.2-14.3C291.4 6.8 280.3 0 268.2 0L147.8 0zM384 128L32 128l0 336c0 26.5 21.5 48 48 48l256 0c26.5 0 48-21.5 48-48l0-336zM160 208l0 224c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-224c0-8.8 7.2-16 16-16s16 7.2 16 16zm128 0l0 224c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-224c0-8.8 7.2-16 16-16s16 7.2 16 16zm192-80c-17.7 0-32 14.3-32 32s14.3 32 32 32l128 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-128 0zm0 128c-17.7 0-32 14.3-32 32s14.3 32 32 32l96 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-96 0zM448 416c0 17.7 14.3 32 32 32l32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0c-17.7 0-32 14.3-32 32z";
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
    exports.faTrashCanList = exports.definition;
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

// src/_entries/faTrashCanList.js
var import_faTrashCanList = __toESM(require_faTrashCanList());
var export_faTrashCanList = import_faTrashCanList.definition;
export {
  export_faTrashCanList as faTrashCanList
};
