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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTrumpet.js
var require_faTrumpet = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTrumpet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "trumpet";
    var width = 640;
    var height = 512;
    var aliases = [127930];
    var unicode = "f8e3";
    var svgPathData = "M628.7 64.7C635.4 66.8 640 73 640 80l0 288c0 7-4.6 13.2-11.3 15.3s-14-.4-17.9-6.2c0 0 0 0 0 0c0 0 0 0 0 0l-.1-.2-.7-.9c-.6-.8-1.5-2.1-2.8-3.8c-2.5-3.3-6.2-8.1-11-13.9c-9.6-11.6-23.6-26.9-40.8-42.3c-35-31.1-81.3-60-131.4-60l-24 0 0 126c27.6-7.1 48-32.2 48-62l0-29.2c22 5 43.7 16.4 64 31.2c-1.1 69.8-58 126-128 126l-95.9 0c0 0 0 0-.1 0l-96 0c-70.7 0-128-57.3-128-128c0-23.3 6.2-45.2 17.1-64l-34.5 0c-3.3 9.3-12.2 16-22.6 16c-13.3 0-24-10.7-24-24l0-48c0-13.3 10.7-24 24-24c10.4 0 19.3 6.7 22.6 16L176 192l0-32c0-13.3 10.7-24 24-24s24 10.7 24 24l0 32 40 0 0-32c0-13.3 10.7-24 24-24s24 10.7 24 24l0 32 40 0 0-32c0-13.3 10.7-24 24-24s24 10.7 24 24l0 32 24 0c50.1 0 96.4-28.8 131.4-60c17.3-15.3 31.2-30.7 40.8-42.3c4.8-5.8 8.5-10.6 11-13.9c1.2-1.7 2.2-2.9 2.8-3.8l.7-.9 .1-.2c0 0 0 0 0 0c0 0 0 0 0 0c4-5.8 11.2-8.3 17.9-6.2zM312 384l40 0 0-128-40 0 0 128zM224 256l0 128 40 0 0-128-40 0zm-96 64c0 29.8 20.4 54.9 48 62l0-124c-27.6 7.1-48 32.2-48 62z";
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
    exports.faTrumpet = exports.definition;
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

// src/_entries/faTrumpet.js
var import_faTrumpet = __toESM(require_faTrumpet());
var export_faTrumpet = import_faTrumpet.definition;
export {
  export_faTrumpet as faTrumpet
};
