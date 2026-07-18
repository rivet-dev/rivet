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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLeafOak.js
var require_faLeafOak = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLeafOak.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "leaf-oak";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f6f7";
    var svgPathData = "M429.7 268.6l40.9-10.2c24.4-6.1 41.4-28 41.4-53.1c0-18.3-9.1-35.4-24.4-45.5l-14.3-9.5c-5.6-3.8-8.3-10.6-6.6-17.2l5-20.1c5.1-20.4-.9-42-15.8-56.9s-36.5-20.9-56.9-15.8l-20.1 5c-6.6 1.6-13.4-1-17.2-6.6l-9.5-14.3C342.1 9.1 325 0 306.7 0c-25.1 0-47 17.1-53.1 41.4L243.4 82.3c-1.5 6-9 8.1-13.4 3.7L208 64c-17.7-17.7-46.3-17.7-64 0c-10.1 10.1-14.8 24.3-12.8 38.4l10.7 75.1c.8 5.6-5.4 9.5-10.1 6.4C111 170 83.3 172.7 65.6 190.4l-3.6 3.6c-17.9 17.9-22.3 45.2-11 67.8L66.5 293C75.4 310.7 80 330.3 80 350.2l0 35.2L229.6 235.8c6.2-6.2 16.4-6.2 22.6 0c5.4 5.4 6.1 13.6 2.2 19.8L39 471c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0l73-73 15.8 0c19.9 0 39.5 4.6 57.2 13.5l31.1 15.5c22.6 11.3 49.9 6.9 67.8-11l3.6-3.6c17.7-17.7 20.4-45.4 6.6-66.2c-3.1-4.7 .8-10.9 6.4-10.1l75.1 10.7c14.1 2 28.3-2.7 38.4-12.8c17.7-17.7 17.7-46.3 0-64l-22-22c-4.4-4.4-2.3-11.9 3.7-13.4z";
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
    exports.faLeafOak = exports.definition;
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

// src/_entries/faLeafOak.js
var import_faLeafOak = __toESM(require_faLeafOak());
var export_faLeafOak = import_faLeafOak.definition;
export {
  export_faLeafOak as faLeafOak
};
