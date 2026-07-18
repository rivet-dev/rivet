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

// src/node_modules/@fortawesome/free-brands-svg-icons/faWaze.js
var require_faWaze = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faWaze.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "waze";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f83f";
    var svgPathData = "M502.6 201.7c14.5 85.9-30.9 167.9-113.2 208.1 13 34.1-12.4 70.2-48.3 70.2-13.2 0-26-5.1-35.6-14.2s-15.3-21.6-16-34.8c-6.4 .2-64.2 0-76.3-.6-.3 6.8-1.9 13.5-4.7 19.6s-6.9 11.7-11.9 16.3-10.8 8.2-17.2 10.5-13.2 3.4-19.9 3.1c-33.9-1.4-58-34.8-47-67.9-37.2-13.1-72.5-34.9-99.6-70.8-13-17.3-.5-41.8 20.8-41.8 46.3 0 32.2-54.2 43.2-110.3 18.4-93.9 116.8-157.1 211.7-157.1 102.5 0 197.2 70.7 214.1 169.7zM373.9 388.3c42-19.2 81.3-56.7 96.3-102.1 40.5-123.1-64.2-228-181.7-228-83.4 0-170.3 55.4-186.1 136-9.5 48.9 5 131.4-68.7 131.4 24.9 33.1 58.3 52.6 93.7 64 24.7-21.8 63.9-15.5 79.8 14.3 14.2 1 79.2 1.2 87.9 .8 3.5-6.9 8.5-12.9 14.7-17.5s13.2-7.9 20.8-9.5 15.4-1.4 22.9 .4 14.5 5.3 20.5 10.2zM205.5 187.1c0-34.7 50.8-34.7 50.8 0s-50.8 34.7-50.8 0zm116.6 0c0-34.7 50.9-34.7 50.9 0s-50.9 34.8-50.9 0zM199.5 257.8c-3.4-16.9 22.2-22.2 25.6-5.2l.1 .3c4.1 21.4 29.8 44 64.1 43.1 35.7-.9 59.3-22.2 64.1-42.8 4.5-16.1 28.6-10.4 25.5 6-5.2 22.2-31.2 62-91.5 62.9-42.6 0-80.9-27.8-87.9-64.2l0 0z";
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
    exports.faWaze = exports.definition;
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

// src/_entries/faWaze.js
var import_faWaze = __toESM(require_faWaze());
var export_faWaze = import_faWaze.definition;
export {
  export_faWaze as faWaze
};
