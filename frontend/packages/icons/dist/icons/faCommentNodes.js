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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCommentNodes.js
var require_faCommentNodes = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCommentNodes.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "comment-nodes";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e696";
    var svgPathData = "M480 208C503.7 208 525.6 215.3 543.7 227.9 536.9 100.9 425 0 288 0 146.6 0 32 107.5 32 240 32 294.3 51.2 344.3 83.6 384.5L34.8 476.8c-4.8 9-3.3 20 3.6 27.5s17.8 9.8 27.1 5.8l118.4-50.7c27.2 11.4 56.9 18.4 88.1 20.2 .1-18.9 4.9-38 15-55.5 17.1-29.7 45.8-48.5 77-54.2l9.2-16c-3.4-10.7-5.2-22-5.2-33.8 0-61.9 50.1-112 112-112zM328.6 448c-17.7 30.6-7.2 69.7 23.4 87.4s69.7 7.2 87.4-23.4c1.5-2.6 2.8-5.3 3.9-8l73.3 0c1.1 2.7 2.4 5.4 3.9 8 17.7 30.6 56.8 41.1 87.4 23.4s41.1-56.8 23.4-87.4c-13.4-23.2-39.1-34.8-64-31.4l-17.6-30.7c-11 11.7-25 20.6-40.6 25.6l16.5 28.9c-3.8 4.8-6.8 10-9 15.6l-73.4 0c-2.2-5.6-5.3-10.8-9-15.6l33-57.7c4.1 .8 8.4 1.3 12.8 1.3 35.3 0 64-28.7 64-64s-28.7-64-64-64-64 28.7-64 64c0 13.4 4.1 25.8 11.2 36.1l-34.6 60.5c-25-3.4-50.6 8.3-64 31.4z";
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
    exports.faCommentNodes = exports.definition;
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

// src/_entries/faCommentNodes.js
var import_faCommentNodes = __toESM(require_faCommentNodes());
var export_faCommentNodes = import_faCommentNodes.definition;
export {
  export_faCommentNodes as faCommentNodes
};
