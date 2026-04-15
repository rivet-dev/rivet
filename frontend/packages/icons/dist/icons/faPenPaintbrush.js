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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPenPaintbrush.js
var require_faPenPaintbrush = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPenPaintbrush.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pen-paintbrush";
    var width = 576;
    var height = 512;
    var aliases = ["pencil-paintbrush"];
    var unicode = "f618";
    var svgPathData = "M95.6 16.4L226.3 147.1l-79.2 79.2L16.4 95.6c-21.9-21.9-21.9-57.3 0-79.2s57.3-21.9 79.2 0zM400 512c-53.6 0-98.5-37.7-109.4-88.1L423.9 290.6c50.3 11 88.1 55.8 88.1 109.4c0 3.9-.2 7.8-.6 11.6c-1.8 17.5 10.2 36.4 27.8 36.4l4.8 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-144 0zM501.3 19.3l23.4 23.4c25 25 25 65.5 0 90.5l-56.4 56.4L354.3 75.7l56.4-56.4c25-25 65.5-25 90.5 0zM76.9 353.2L331.7 98.3 445.7 212.3 190.8 467.1c-6.7 6.7-15.1 11.6-24.2 14.2l-104 29.7c-8.4 2.4-17.4 .1-23.6-6.1s-8.5-15.2-6.1-23.6l29.7-104c2.6-9.2 7.5-17.5 14.2-24.2z";
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
    exports.faPenPaintbrush = exports.definition;
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

// src/_entries/faPenPaintbrush.js
var import_faPenPaintbrush = __toESM(require_faPenPaintbrush());
var export_faPenPaintbrush = import_faPenPaintbrush.definition;
export {
  export_faPenPaintbrush as faPenPaintbrush
};
