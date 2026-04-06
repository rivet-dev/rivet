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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareThisWayUp.js
var require_faSquareThisWayUp = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareThisWayUp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-this-way-up";
    var width = 448;
    var height = 512;
    var aliases = ["box-up"];
    var unicode = "f49f";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM80 384l288 0c8.8 0 16 7.2 16 16s-7.2 16-16 16L80 416c-8.8 0-16-7.2-16-16s7.2-16 16-16zM265.4 192c-9.6 0-17.4-7.8-17.4-17.4c0-4.3 1.6-8.4 4.4-11.6L306.7 102c3.4-3.8 8.2-6 13.3-6s9.9 2.2 13.3 6l54.3 61.1c2.8 3.2 4.4 7.3 4.4 11.6c0 9.6-7.8 17.4-17.4 17.4L352 192l0 112c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-112-22.6 0zm-192 0c-9.6 0-17.4-7.8-17.4-17.4c0-4.3 1.6-8.4 4.4-11.6L114.7 102c3.4-3.8 8.2-6 13.3-6s9.9 2.2 13.3 6l54.3 61.1c2.8 3.2 4.4 7.3 4.4 11.6c0 9.6-7.8 17.4-17.4 17.4L160 192l0 112c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16l0-112-22.6 0z";
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
    exports.faSquareThisWayUp = exports.definition;
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

// src/_entries/faBoxUp.js
var import_faSquareThisWayUp = __toESM(require_faSquareThisWayUp());
var export_faBoxUp = import_faSquareThisWayUp.definition;
export {
  export_faBoxUp as faBoxUp
};
