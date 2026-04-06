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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCloudsSun.js
var require_faCloudsSun = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCloudsSun.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "clouds-sun";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f746";
    var svgPathData = "M294.2 1.2c5.1 2.1 8.7 6.7 9.6 12.1l14.1 84.7 30.7 5.1c-26.4 10.6-47 32.6-55.7 59.9C276.8 132.7 244.8 112 208 112c-53 0-96 43-96 96s43 96 96 96c8.1 0 15.9-1 23.4-2.9c-36.6 18.1-63.3 53.1-69.8 94.9l-24.4 17c-4.5 3.2-10.3 3.8-15.4 1.6s-8.7-6.7-9.6-12.1L98.1 317.9 13.4 303.8c-5.4-.9-10-4.5-12.1-9.6s-1.5-10.9 1.6-15.4L52.5 208 2.9 137.2c-3.2-4.5-3.8-10.3-1.6-15.4s6.7-8.7 12.1-9.6L98.1 98.1l14.1-84.7c.9-5.4 4.5-10 9.6-12.1s10.9-1.5 15.4 1.6L208 52.5 278.8 2.9c4.5-3.2 10.3-3.8 15.4-1.6zM144 208a64 64 0 1 1 128 0 64 64 0 1 1 -128 0zm450.6 75c-18.9-35.1-56-59-98.7-59c-12.9 0-25.3 2.2-36.8 6.2C435.9 206.6 403.7 192 368 192c-16.7 0-32.6 3.2-47.3 9c-.4-2.9-.6-5.9-.6-8.9c0-35.3 28.6-64 64-64c7.4 0 14.6 1.3 21.2 3.6c11.5-30.1 40.6-51.6 74.8-51.6c38.9 0 71.3 27.8 78.5 64.6c3.1-.4 6.3-.6 9.5-.6c39.8 0 72 32.2 72 72c0 30.4-18.8 56.3-45.4 66.9zm45.3 148.9c0 44.2-35.8 80-80 80l-271.9 0c-53 0-96-43-96-96c0-47.6 34.6-87 80-94.6l0-1.3c0-53 43-96 96-96c34.9 0 65.4 18.6 82.2 46.4c13-9.1 28.8-14.4 45.8-14.4c44.2 0 80 35.8 80 80c0 5.9-.6 11.7-1.9 17.2c37.4 6.7 65.8 39.4 65.8 78.7z";
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
    exports.faCloudsSun = exports.definition;
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

// src/_entries/faCloudsSun.js
var import_faCloudsSun = __toESM(require_faCloudsSun());
var export_faCloudsSun = import_faCloudsSun.definition;
export {
  export_faCloudsSun as faCloudsSun
};
