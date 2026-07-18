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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHousePersonReturn.js
var require_faHousePersonReturn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHousePersonReturn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "house-person-return";
    var width = 640;
    var height = 512;
    var aliases = ["house-person-arrive", "house-return"];
    var unicode = "e011";
    var svgPathData = "M218.3 8.5c12.3-11.3 31.2-11.3 43.4 0L388.9 125.9c-19 14.8-34.4 34.5-44.1 57.7l-10.7 25.8-10.7 5.4c-31.6 15.8-44.4 54.3-28.6 85.9c15.4 30.9 52.5 43.8 83.6 29.7l-6.8 7.5c-9 9.8-15.4 21.7-18.6 34.7L342 416l-230 0c-26.5 0-48-21.5-48-48l0-112-32 0c-13.2 0-25-8.1-29.8-20.3s-1.6-26.2 8.1-35.2l208-192zM128 208l0 64c0 8.8 7.2 16 16 16l64 0c8.8 0 16-7.2 16-16l0-64c0-8.8-7.2-16-16-16l-64 0c-8.8 0-16 7.2-16 16zM432 0a48 48 0 1 1 0 96 48 48 0 1 1 0-96zm84.3 200.5c-1-.4-1.9-.8-2.9-1.2l16.9 63.5c5.6 21.1 .1 43.6-14.7 59.7l-70.7 77.1-22 88.1c-4.3 17.1-21.7 27.6-38.8 23.3s-27.6-21.7-23.3-38.8l23-92.1c1.9-7.8 5.8-14.9 11.2-20.8l49.5-54-19.3-65.5-9.6 23c-4.4 10.6-12.5 19.3-22.8 24.5l-26.7 13.3c-15.8 7.9-35 1.5-42.9-14.3s-1.5-35 14.3-42.9L359 232.7l15.3-36.8c17.1-41.1 57.3-67.9 101.9-67.9c22.8 0 45.3 4.8 66.1 14l8 3.5c32.9 14.6 58.1 42.4 69.4 76.5l2.6 7.8c5.6 16.8-3.5 34.9-20.2 40.5s-34.9-3.5-40.5-20.2l-2.6-7.8c-5.7-17.1-18.3-30.9-34.7-38.2l-8-3.5zm30 135.1l25 62.4 59.4 59.4c12.5 12.5 12.5 32.8 0 45.3s-32.8 12.5-45.3 0L523.7 441c-4.6-4.6-8.2-10.1-10.6-16.1l-14.5-36.2 40.7-44.4c2.5-2.7 4.8-5.6 7-8.6z";
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
    exports.faHousePersonReturn = exports.definition;
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

// src/_entries/faHouseReturn.js
var import_faHousePersonReturn = __toESM(require_faHousePersonReturn());
var export_faHouseReturn = import_faHousePersonReturn.definition;
export {
  export_faHouseReturn as faHouseReturn
};
