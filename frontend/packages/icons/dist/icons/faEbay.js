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

// src/node_modules/@fortawesome/free-brands-svg-icons/faEbay.js
var require_faEbay = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faEbay.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "ebay";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f4f4";
    var svgPathData = "M606.5 189.5l-54.8 109.9-54.9-109.9-37.5 0 10.9 20.6c-11.5-19-35.9-26-63.3-26-31.8 0-67.9 8.7-71.5 43.1l33.7 0c1.4-13.8 15.7-21.8 35-21.8 26 0 41 9.6 41 33l0 3.4c-12.7 0-28 .1-41.7 .4-42.4 .9-69.6 10-76.7 34.4 1-5.2 1.5-10.6 1.5-16.2 0-52.1-39.7-76.2-75.4-76.2-21.3 0-43 5.5-58.7 24.2l0-80.6-32.1 0 0 169.5c0 10.3-.6 22.9-1.1 33.1l31.5 0c.7-6.3 1.1-12.9 1.1-19.5 13.6 16.6 35.4 24.9 58.7 24.9 36.9 0 64.9-21.9 73.3-54.2-.5 2.8-.7 5.8-.7 9 0 24.1 21.1 45 60.6 45 26.6 0 45.8-5.7 61.9-25.5 0 6.6 .3 13.3 1.1 20.2l29.8 0c-.7-8.2-1-17.5-1-26.8l0-65.6c0-9.3-1.7-17.2-4.8-23.8l61.5 116.1-28.5 54.1 35.9 0 99.2-194.8-34 0zM244.2 313.8c-29.6 0-50.2-21.5-50.2-53.8 0-32.4 20.6-53.8 50.2-53.8 29.8 0 50.2 21.4 50.2 53.8 0 32.3-20.4 53.8-50.2 53.8zm200.9-47.3c0 30-17.9 48.4-51.6 48.4-25.1 0-35-13.4-35-25.8 0-19.1 18.1-24.4 47.2-25.3 13.1-.5 27.6-.6 39.4-.6l0 3.3zM33.2 268.1l128.8 0 0-8.5c0-51.7-33.1-75.4-78.4-75.4-56.8 0-83 30.8-83 77.6 0 42.5 25.3 74 82.5 74 31.4 0 68-11.7 74.4-46.1l-33.1 0c-12 35.8-87.7 36.7-91.2-21.6zm95-21.4l-94.4 0c6.9-56.6 92.1-54.7 94.4 0z";
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
    exports.faEbay = exports.definition;
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

// src/_entries/faEbay.js
var import_faEbay = __toESM(require_faEbay());
var export_faEbay = import_faEbay.definition;
export {
  export_faEbay as faEbay
};
