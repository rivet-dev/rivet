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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGarageCar.js
var require_faGarageCar = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGarageCar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "garage-car";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e00a";
    var svgPathData = "M0 488L0 171.3c0-26.2 15.9-49.7 40.2-59.4L308.1 4.8c7.6-3.1 16.1-3.1 23.8 0L599.8 111.9c24.3 9.7 40.2 33.3 40.2 59.4L640 488c0 13.3-10.7 24-24 24l-48 0c-13.3 0-24-10.7-24-24l0-264c0-17.7-14.3-32-32-32l-384 0c-17.7 0-32 14.3-32 32l0 264c0 13.3-10.7 24-24 24l-48 0c-13.3 0-24-10.7-24-24zM254.3 293.4L242.2 328l155.6 0-12.1-34.6c-1.1-3.2-4.2-5.4-7.6-5.4l-116.3 0c-3.4 0-6.4 2.2-7.6 5.4zM188.9 335L209 277.5c7.9-22.5 29.1-37.5 52.9-37.5l116.3 0c23.8 0 45 15.1 52.9 37.5L451.1 335c17.2 9.5 28.9 27.9 28.9 49l0 96c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-16-192 0 0 16c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-96c0-21.1 11.7-39.5 28.9-49zM240 424a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm184-24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faGarageCar = exports.definition;
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

// src/_entries/faGarageCar.js
var import_faGarageCar = __toESM(require_faGarageCar());
var export_faGarageCar = import_faGarageCar.definition;
export {
  export_faGarageCar as faGarageCar
};
