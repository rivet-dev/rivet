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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLeafMaple.js
var require_faLeafMaple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLeafMaple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "leaf-maple";
    var width = 512;
    var height = 512;
    var aliases = [127809];
    var unicode = "f6f6";
    var svgPathData = "M339.8 7.5l15.8 31.6c7.1 14.2 23.6 21.1 38.7 16l67.6-22.5c1.4-.5 2.9-.7 4.4-.7c7.6 0 13.8 6.2 13.8 13.8c0 1.5-.2 2.9-.7 4.4l-22.5 67.6c-5 15.1 1.8 31.6 16 38.7l31.6 15.8c4.6 2.3 7.5 7 7.5 12.2c0 4.7-2.4 9.1-6.4 11.6l-97.9 61.2c-6.8 4.3-3.8 14.8 4.2 14.8l38.3 0c7.6 0 13.8 6.2 13.8 13.8c0 1.5-.2 2.9-.7 4.4l-8.5 25.5c-4.1 12.3-.3 25.9 9.5 34.4l34.3 29.4c3.4 2.9 5.4 7.2 5.4 11.7c0 5.5-3 10.6-7.8 13.4l-28.9 16.5c-11.6 6.6-17.9 19.8-15.7 33l4.2 25.1c.1 .6 .2 1.3 .2 1.9c0 7.6-7.2 13.2-14.6 11.2l-155-40.4c-8.2-2.1-16.6 2.4-19.2 10.4l-8.5 25.4c-1.7 5-6.3 8.4-11.6 8.4c-4.4 0-8.5-2.4-10.7-6.3l-53-95.3L73 505c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9L239.3 270.8c3.6-6.1 2.7-14.1-2.5-19.4c-6.2-6.2-16.4-6.2-22.6 0l-84 84L22.3 275.5c-3.9-2.2-6.3-6.2-6.3-10.7c0-5.3 3.4-9.9 8.4-11.6l25.4-8.5c8-2.7 12.5-11.1 10.4-19.2L19.8 70.6C17.9 63.2 23.4 56 31 56c.6 0 1.3 .1 1.9 .2L58 60.3c13.2 2.2 26.4-4.1 33-15.7l16.5-28.9C110.3 11 115.4 8 120.9 8c4.5 0 8.8 2 11.7 5.4L162 47.7c8.5 9.9 22.1 13.6 34.4 9.5l25.5-8.5c1.4-.5 2.9-.7 4.4-.7c7.6 0 13.8 6.2 13.8 13.8l0 38.3c0 8 10.5 11.1 14.8 4.2L316 6.4c2.5-4 6.9-6.4 11.6-6.4c5.2 0 9.9 2.9 12.2 7.5z";
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
    exports.faLeafMaple = exports.definition;
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

// src/_entries/faLeafMaple.js
var import_faLeafMaple = __toESM(require_faLeafMaple());
var export_faLeafMaple = import_faLeafMaple.definition;
export {
  export_faLeafMaple as faLeafMaple
};
