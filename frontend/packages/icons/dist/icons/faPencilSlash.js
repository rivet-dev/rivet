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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPencilSlash.js
var require_faPencilSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPencilSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pencil-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e215";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7L409.6 295.8l42.1-42.1L474.3 231l11.3-11.3-33.9-33.9-62.1-62.1L355.7 89.8l-11.3 11.3-22.6 22.6-57.8 57.8L38.8 5.1zM306.2 214.7l50.5-50.5c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6l-47.8 47.8-25.4-19.9zM195.5 250l-72.9 72.9c-10.4 10.4-18 23.3-22.2 37.4L65 480.7c-2.5 8.4-.2 17.5 6.1 23.7s15.3 8.5 23.7 6.1l120.3-35.4c14.1-4.2 27-11.8 37.4-22.2l88.3-88.3-77.9-61.4-27.6 27.6c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6l24.9-24.9L195.5 250zM224 399.4l-9.1 22.7c-4 3.1-8.5 5.4-13.3 6.9l-78.1 23 23-78.1c1.4-4.9 3.8-9.4 6.9-13.3l22.7-9.1 0 32c0 8.8 7.2 16 16 16l32 0zM426.7 18.7L412.3 33.2 389.7 55.8 378.3 67.1l33.9 33.9 62.1 62.1 33.9 33.9 11.3-11.3 22.6-22.6 14.5-14.5c25-25 25-65.5 0-90.5L517.3 18.7c-25-25-65.5-25-90.5 0z";
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
    exports.faPencilSlash = exports.definition;
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

// src/_entries/faPencilSlash.js
var import_faPencilSlash = __toESM(require_faPencilSlash());
var export_faPencilSlash = import_faPencilSlash.definition;
export {
  export_faPencilSlash as faPencilSlash
};
