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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGingerbreadMan.js
var require_faGingerbreadMan = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGingerbreadMan.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "gingerbread-man";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f79d";
    var svgPathData = "M296.9 158.5c-5.2 6-1.2 17.5 6.7 17.5l64.4 0c35.3 0 64 28.7 64 64s-28.7 64-64 64l-20.9 0c-19 0-29.4 22.2-17.2 36.7L385.2 407c22.6 27.2 19 67.5-8.2 90.1s-67.5 19-90.1-8.2l-45.6-54.8c-9-10.7-25.5-10.7-34.4 0L161.2 489c-22.6 27.2-63 30.8-90.1 8.2s-30.8-63-8.2-90.1l55.2-66.3c12.2-14.6 1.8-36.7-17.2-36.7L80 304c-35.3 0-64-28.7-64-64s28.7-64 64-64l64.4 0c7.9 0 11.9-11.5 6.7-17.5C136.7 141.7 128 119.9 128 96c0-53 43-96 96-96s96 43 96 96c0 23.9-8.7 45.7-23.1 62.5zM192 96a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm80-16a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM224 240a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm16 112a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm-16-48a16 16 0 1 0 0-32 16 16 0 1 0 0 32z";
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
    exports.faGingerbreadMan = exports.definition;
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

// src/_entries/faGingerbreadMan.js
var import_faGingerbreadMan = __toESM(require_faGingerbreadMan());
var export_faGingerbreadMan = import_faGingerbreadMan.definition;
export {
  export_faGingerbreadMan as faGingerbreadMan
};
